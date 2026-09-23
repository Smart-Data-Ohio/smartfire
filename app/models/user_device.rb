# The browsers an account has signed in from, keyed by the long-lived
# signed device cookie (see Authentication#ensure_device_cookie), not
# the IP. Rows outlive sessions: signing out everywhere does not make
# the next sign-in "first ever". The stored user agent is refreshed on
# every sign-in so browser upgrades do not alert; only a cookie the
# account never used is a new device.
class UserDevice < ApplicationRecord
  belongs_to :user

  validates :device_id, presence: true

  # Records a sign-in. Returns :first_seen when the account has no
  # devices yet, :known when this device was seen before, and
  # :new_device when the account knows other devices but not this one.
  # A lost creation race reads as known: the winning row proves the
  # device, and a concurrent first sign-in from two browsers alerting
  # once would be noise, not signal.
  def self.record_sign_in!(user:, device_id:, user_agent:)
    return :unknown if device_id.blank?

    devices = user.user_devices
    if devices.none?
      create!(user: user, device_id: device_id, user_agent: user_agent)
      :first_seen
    elsif devices.where(device_id: device_id).exists?
      devices.where(device_id: device_id).update_all(user_agent: user_agent, updated_at: Time.current)
      :known
    else
      create!(user: user, device_id: device_id, user_agent: user_agent)
      :new_device
    end
  rescue ActiveRecord::RecordNotUnique
    :known
  end
end
