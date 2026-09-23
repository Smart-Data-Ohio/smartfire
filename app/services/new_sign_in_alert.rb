# In-app (and, when mail is configured, email) alert for sign-ins from a
# device or browser the account has not used before. Keyed on the
# long-lived signed device cookie recorded on UserDevice, not the IP
# alone, so travel and carrier NAT do not alert. The first-ever sign-in
# never alerts: there is no established device to compare against, and
# the owner is necessarily the one signing in.
#
# The email goes out from ActivityItem's after_create_commit, so a rolled
# back sign-in (the Google callback signs in inside a transaction) sends
# nothing; the inbox item is created here, synchronously with sign-in.
class NewSignInAlert
  def self.deliver_if_new_device(user, session)
    return if session.device_id.blank?

    result = UserDevice.record_sign_in!(user: user, device_id: session.device_id, user_agent: session.user_agent)
    return unless result == :new_device

    ActivityItem.create!(user: user, source: session, event_type: "new_sign_in")
  end
end
