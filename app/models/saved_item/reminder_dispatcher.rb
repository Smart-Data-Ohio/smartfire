class SavedItem::ReminderDispatcher
  class << self
    # Called on an interval by Periodic::Runner. Claiming through
    # reminded_at keeps a second run (or a second runner) from firing
    # the same reminder twice.
    def dispatch_due!(now: Time.current)
      due_items(now).find_each do |saved_item|
        dispatch_item!(saved_item, now:)
      rescue => error
        Rails.logger.error "Saved item reminder failed for saved item #{saved_item.id}: #{error.class}: #{error.message}"
      end
    end

    private
      def due_items(now)
        SavedItem.due_reminders(now)
          .joins(:user, message: :room)
          .merge(User.active.without_bots)
          .merge(Room.alive)
      end

      def dispatch_item!(saved_item, now:)
        claimed = saved_item.with_lock do
          if saved_item.reminded_at.present?
            false
          elsif saved_item.message.room.memberships.exists?(user_id: saved_item.user_id)
            saved_item.transition_reminder_item!
            saved_item.update!(reminded_at: now)
            true
          else
            # The saver lost room access: claim without notifying, so
            # the reminder never leaks message content to a non-member.
            saved_item.update!(reminded_at: now)
            false
          end
        end

        SavedItem::ReminderPushJob.perform_later(saved_item) if claimed
      end
  end
end
