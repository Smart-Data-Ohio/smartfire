class Session < ApplicationRecord
  ACTIVITY_REFRESH_RATE = 1.hour

  has_secure_token

  belongs_to :user
  has_many :workspace_presence_leases, dependent: :delete_all

  before_destroy -> { HuddleGrant.revoke_for_session!(self) }
  before_create { self.last_active_at ||= Time.now }

  def self.start!(user_agent:, ip_address:, device_id: nil)
    create! user_agent: user_agent, ip_address: ip_address, device_id: device_id
  end

  def resume(user_agent:, ip_address:)
    if last_active_at.before?(ACTIVITY_REFRESH_RATE.ago)
      update! user_agent: user_agent, ip_address: ip_address, last_active_at: Time.now
    end
  end

  # Administrator sessions expire after the configured idle time (see
  # config/initializers/session_lifetimes.rb); member sessions keep the
  # current never-expire lifetime. Checked on every resume and on cable
  # connect, so expiry takes effect on the next request.
  def expired?
    user.administrator? && last_active_at.before?(admin_idle_timeout.ago)
  end

  def admin_idle_timeout
    Rails.configuration.x.admin_session_idle_timeout
  end

  def browser_name
    # Edge first, matched on the raw string: its UA also contains a
    # Chrome token, which is what UserAgent parses as the browser.
    ua = user_agent.to_s
    platform = ApplicationPlatform.new(ua)
    if ua.include?("Edg/")
      "Edge"
    elsif platform.chrome?
      "Chrome"
    elsif platform.firefox?
      "Firefox"
    elsif platform.safari?
      "Safari"
    else
      "Unknown browser"
    end
  end

  def os_name
    ApplicationPlatform.new(user_agent).operating_system.presence || "Unknown device"
  end

  # "Chrome on macOS": the human description used by the new-sign-in
  # alert and the "Your sessions" page.
  def device_description
    "#{browser_name} on #{os_name}"
  end
end
