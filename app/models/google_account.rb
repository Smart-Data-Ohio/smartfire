# A member's connected Google account for one-way event publishing.
# Connecting is the explicit opt-in; nothing is published without a row here.
# Tokens are encrypted at rest (see active_record_encryption initializer).
class GoogleAccount < ApplicationRecord
  UNREADABLE_TOKEN_REASON = "The stored token could not be read; reconnect"

  belongs_to :user

  encrypts :refresh_token, :access_token

  validates :user_id, uniqueness: true
  validates :email, presence: true

  # False once Google refuses a refresh with invalid_grant; the row stays so
  # the profile can offer a reconnect instead of a first-time connect.
  def connected?
    disconnected_reason.blank?
  end

  # An undecryptable token (rotated or lost encryption key) reads as
  # unusable rather than raising: the account is marked disconnected so
  # the profile offers a reconnect instead of a first-time connect.
  def usable?
    connected? && refresh_token.present?
  rescue ActiveRecord::Encryption::Errors::Decryption
    mark_disconnected!(UNREADABLE_TOKEN_REASON)
    false
  end

  # True when the stored OAuth grant includes the Calendar scope. Existing
  # rows have null scopes (calendar only, from before scopes were stored),
  # so a blank string still counts as granted; only an explicit grant
  # without calendar.events fails.
  def calendar?
    scopes.blank? || scopes.to_s.split.include?(Google::Client::CALENDAR_SCOPE)
  end

  # True when the stored OAuth grant includes the Drive metadata scope.
  # Existing rows have null scopes (calendar only).
  def drive?
    scopes.to_s.split.include?(Google::Client::DRIVE_SCOPE)
  end

  def access_token_expired?
    access_token.blank? || access_token_expires_at.blank? || access_token_expires_at <= Time.current
  end

  # Token snapshot for post-disconnect cleanup, when this row is about to
  # be destroyed. Nil when the tokens cannot be read (decryption failure)
  # or were never stored: there is nothing cleanup could authenticate.
  def cleanup_snapshot
    return nil if refresh_token.blank?

    {
      access_token:, refresh_token:,
      access_token_expires_at:
    }
  rescue ActiveRecord::Encryption::Errors::Decryption
    nil
  end

  def mark_disconnected!(reason)
    update!(disconnected_reason: reason)
  end
end
