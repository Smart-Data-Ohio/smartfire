# Whether an incoming call rings audibly for a user. The banner always
# shows; this only gates the ringtone and the system Notification, and
# follows Notifications::Policy: do-not-disturb (including timed DND)
# silences the ring unless the caller is on the recipient's "Allow during
# DND" list. Tests may override the decision with `quiet_check`.
class Huddle::RingPolicy
  class << self
    attr_writer :quiet_check

    def ring?(user, caller: nil)
      if (check = @quiet_check)
        !check.call(user)
      else
        Notifications::Policy.new(recipient: user, sender: caller, kind: :huddle).sound?
      end
    end
  end
end
