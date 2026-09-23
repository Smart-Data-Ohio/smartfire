# A member's linked GitHub identity for PR write actions (comments, reviews).
# Holds a per-user fine-grained personal access token, validated against
# GET /user at link time; the token is encrypted at rest (see
# active_record_encryption initializer) and is never logged or rendered back.
class GithubConnectedAccount < ApplicationRecord
  UNREADABLE_TOKEN_REASON = "The stored token could not be read; link it again"
  REPOSITORY_ACCESS_TTL = 10.minutes

  belongs_to :user

  encrypts :access_token, :refresh_token

  validates :user_id, uniqueness: true
  validates :github_login, presence: true
  validates :token_source, inclusion: { in: %w[ pat app ] }

  after_save :claim_verified_login, if: :connected?

  # True for tokens issued through the workspace GitHub App (short-lived
  # with refresh); false for pasted personal access tokens, which stay as
  # the migration fallback.
  def app_token?
    token_source == "app"
  end

  # False once GitHub refuses the token with 401; the row stays so the
  # profile can offer a reconnect instead of a first-time connect.
  def connected?
    disconnected_reason.blank?
  end

  # An undecryptable token (rotated or lost encryption key) reads as
  # unusable rather than raising: the account is marked disconnected so
  # the profile offers a reconnect instead of a first-time connect.
  def usable?
    connected? && access_token.present?
  rescue ActiveRecord::Encryption::Errors::Decryption
    mark_disconnected!(UNREADABLE_TOKEN_REASON)
    false
  end

  # Whether this linked account can read owner/repo, checked with its own
  # token against GET /repos/{owner}/{repo}. Backs the per-viewer gate for
  # private-repo cards, repository subscriptions, and agent payloads.
  #
  # The decision is cached per member and repository for 10 minutes, both
  # grants and denials, so a page of cards from one repository costs at
  # most one GitHub request per member per window. The key carries
  # updated_at, so linking, relinking, or repairing the account retires
  # that member's cached decisions without enumerating repositories. An
  # unusable account is denied with no request, and a transport failure
  # denies without caching so the next check retries.
  def can_read_repository?(owner, repo)
    return false unless usable?

    token = access_token_for_use
    return false if token.nil?

    owner = owner.to_s.downcase
    repo = repo.to_s.downcase
    # updated_at at float precision: a relink in the same second as a cached
    # denial must still retire it.
    Rails.cache.fetch([ "github_repo_access", user_id, updated_at.to_f, owner, repo ], expires_in: REPOSITORY_ACCESS_TTL) do
      Github::WriteClient.new(token: token).repository_readable?(owner, repo)
    rescue Github::WriteClient::Unauthorized
      mark_disconnected!("GitHub rejected the linked token (401)")
      false
    end
  rescue ActiveRecord::Encryption::Errors::Decryption
    mark_disconnected!(UNREADABLE_TOKEN_REASON)
    false
  rescue Github::WriteClient::Error
    false
  end

  # The access token to use for a GitHub call, refreshing an expired App
  # token first. Nil when the account is not usable or the refresh
  # failed; a failed refresh marks the account disconnected (401-like)
  # or records last_error (transient), never raising.
  def access_token_for_use
    return nil unless usable?
    return nil unless refresh_app_token_if_expired!

    access_token
  rescue ActiveRecord::Encryption::Errors::Decryption
    mark_disconnected!(UNREADABLE_TOKEN_REASON)
    nil
  end

  # Best-effort remote revocation of an App token for disconnect. PATs
  # have no revocation endpoint, so nothing is sent for them. Never
  # raises; disconnect proceeds however revocation goes.
  def revoke_remote_token!
    return unless app_token?

    token = access_token
    Github::App.revoke_token(token) if token.present?
  rescue ActiveRecord::Encryption::Errors::Decryption
    nil
  end

  def app_token_expired?
    app_token? && token_expires_at.present? && token_expires_at <= 60.seconds.from_now
  end

  def mark_disconnected!(reason)
    update!(disconnected_reason: reason)
  end

  private
    # Refreshes an expired App token in place, rotating the stored
    # refresh token. A rejected refresh disconnects the account so the
    # profile offers a reconnect; a transport failure records last_error
    # and keeps the old token, so the next call retries.
    def refresh_app_token_if_expired!
      return true unless app_token_expired?
      return false if refresh_token.blank?

      tokens = Github::App.refresh_access_token(refresh_token: refresh_token)
      update!(
        access_token: tokens["access_token"],
        refresh_token: tokens["refresh_token"].presence || refresh_token,
        token_expires_at: Time.current + tokens["expires_in"].to_i.seconds,
        last_error: nil
      )
      true
    rescue Github::App::Unauthorized
      concurrent_refresh_won? || disconnect_rejected!
    rescue Github::App::Error => error
      update_column(:last_error, error.message.truncate(250))
      false
    end

    # True when a concurrent refresh already rotated this row to a fresh
    # token: GitHub rejects the loser's rotated-out refresh token, so a
    # rejection against a now-fresh row means another process won, and
    # this process must use its token instead of disconnecting.
    def concurrent_refresh_won?
      reload
      connected? && !app_token_expired?
    rescue ActiveRecord::RecordNotFound
      false
    end

    def disconnect_rejected!
      mark_disconnected!("GitHub rejected the linked token (401)") if persisted?
      false
    end
    # GitHub confirmed this login when the token was linked, so it becomes
    # the member's profile login (review requests route by it). Anyone else
    # who merely typed the same login into their profile loses it; a second
    # member whose own linked token confirms the same login keeps it, and
    # this member's profile login is then left unchanged.
    def claim_verified_login
      login = github_login.to_s.strip.downcase
      return if login.blank? || user.github_login == login

      claimants = User.where("LOWER(github_login) = ?", login).where.not(id: user_id).includes(:github_connected_account).to_a
      return if claimants.any? { |claimant| claimant.github_connected_account&.connected? && claimant.github_connected_account.github_login.to_s.casecmp?(login) }

      claimants.each { |claimant| claimant.update_columns(github_login: nil) }
      user.update_columns(github_login: login)
    end
end
