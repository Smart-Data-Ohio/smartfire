# A member's linked GitHub identity for PR write actions (comments, reviews).
# Holds a per-user fine-grained personal access token, validated against
# GET /user at link time; the token is encrypted at rest (see
# active_record_encryption initializer) and is never logged or rendered back.
class GithubConnectedAccount < ApplicationRecord
  UNREADABLE_TOKEN_REASON = "The stored token could not be read; link it again"
  REPOSITORY_ACCESS_TTL = 10.minutes

  belongs_to :user

  encrypts :access_token

  validates :user_id, uniqueness: true
  validates :github_login, presence: true

  after_save :claim_verified_login, if: :connected?

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

    owner = owner.to_s.downcase
    repo = repo.to_s.downcase
    # updated_at at float precision: a relink in the same second as a cached
    # denial must still retire it.
    Rails.cache.fetch([ "github_repo_access", user_id, updated_at.to_f, owner, repo ], expires_in: REPOSITORY_ACCESS_TTL) do
      Github::WriteClient.new(token: access_token).repository_readable?(owner, repo)
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

  def mark_disconnected!(reason)
    update!(disconnected_reason: reason)
  end

  private
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
