# Whether an incoming call rings audibly for a user. The banner always
# shows; this only gates the ringtone and the system Notification. Do-not-
# disturb and quiet hours live in Notifications::Policy on the
# status/notifications branch, which is not on main yet: when it lands,
# integration wires it here with one line (for example in an initializer):
#
#   Huddle::RingPolicy.quiet_check = ->(user) { Notifications::Policy.quiet_now?(user) }
#
# Until then every invitation rings.
class Huddle::RingPolicy
  class << self
    attr_writer :quiet_check

    def ring?(user)
      check = @quiet_check
      return true unless check

      !check.call(user)
    end
  end
end
