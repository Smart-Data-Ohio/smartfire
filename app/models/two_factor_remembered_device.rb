# Server-side record backing the "remember this device" cookie for
# two-step sign-in. The cookie carries a random token; only its digest
# is stored, so a database read alone cannot mint remember cookies.
class TwoFactorRememberedDevice < ApplicationRecord
  REMEMBER_FOR = 30.days
  USER_AGENT_MAX = 512

  belongs_to :user

  validates :token_digest, presence: true, uniqueness: true

  scope :live, -> { where("expires_at > ?", Time.current) }
  scope :recent_first, -> { order(last_used_at: :desc, id: :desc) }

  def self.generate_token
    SecureRandom.hex(32)
  end

  def self.digest(token)
    Digest::SHA256.hexdigest(token.to_s)
  end

  def self.create_for!(user, user_agent:, ip_address:)
    token = generate_token
    device = create!(
      user: user,
      token_digest: digest(token),
      user_agent: user_agent&.truncate(USER_AGENT_MAX),
      ip_address: ip_address,
      expires_at: REMEMBER_FOR.from_now,
      last_used_at: Time.current
    )
    [ device, token ]
  end

  # Finds the caller's live device for the pending user and stamps its
  # use. Returns nil for unknown, foreign, or expired tokens.
  def self.find_valid(token, user)
    return nil if token.blank? || user.nil?

    device = find_by(token_digest: digest(token.to_s.strip), user_id: user.id)
    return nil if device.nil? || device.expired?

    device.touch_last_used!
    device
  end

  def expired?
    expires_at <= Time.current
  end

  def touch_last_used!
    update_columns(last_used_at: Time.current, updated_at: Time.current)
  end
end
