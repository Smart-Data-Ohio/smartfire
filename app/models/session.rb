class Session < ApplicationRecord
  ACTIVITY_REFRESH_RATE = 1.hour

  has_secure_token

  belongs_to :user
  has_many :workspace_presence_leases, dependent: :delete_all
  has_many :two_factor_setup_secrets, dependent: :delete_all

  before_destroy -> { HuddleGrant.revoke_for_session!(self) }
  before_create { self.last_active_at ||= Time.now }

  def self.start!(user_agent:, ip_address:, two_factor_verified: false)
    create! user_agent: user_agent, ip_address: ip_address,
      two_factor_verified_at: (Time.current if two_factor_verified)
  end

  def two_factor_verified?
    two_factor_verified_at.present?
  end

  def mark_two_factor_verified!
    update!(two_factor_verified_at: Time.current) unless two_factor_verified?
  end

  def clear_two_factor_verified!
    update!(two_factor_verified_at: nil) if two_factor_verified?
  end

  def resume(user_agent:, ip_address:)
    if last_active_at.before?(ACTIVITY_REFRESH_RATE.ago)
      update! user_agent: user_agent, ip_address: ip_address, last_active_at: Time.now
    end
  end
end
