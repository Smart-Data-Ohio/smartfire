# A member's linked Fizzy identity for card previews and card actions.
# Holds a per-user personal access token, validated against GET
# /my/identity at link time; the token is encrypted at rest (see
# active_record_encryption initializer) and is never logged or rendered back.
class FizzyConnectedAccount < ApplicationRecord
  UNREADABLE_TOKEN_REASON = "The stored token could not be read; link it again"

  belongs_to :user

  encrypts :access_token

  validates :user_id, uniqueness: true
  validates :fizzy_account_id, presence: true

  # False once Fizzy refuses the token with 401; the row stays so the
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

  def mark_disconnected!(reason)
    update!(disconnected_reason: reason)
  end
end
