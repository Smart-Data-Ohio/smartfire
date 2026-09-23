class ScheduledMessage::Dispatcher
  class << self
    # Called on an interval by Periodic::Runner. Each due row is claimed
    # through claimed_at before posting, so a second run (or a second
    # runner) cannot send it twice. Access is re-checked at send time:
    # rows whose author lost access are dropped, and the author gets an
    # inbox item explaining why.
    def dispatch_due!(now: Time.current)
      due_candidates(now).find_each do |scheduled|
        dispatch_item!(scheduled, now:)
      rescue => error
        Rails.logger.error "Scheduled message failed for scheduled message #{scheduled.id}: #{error.class}: #{error.message}"
      end
    end

    # Sends one row immediately (the view's "Send now"). Returns true
    # when posted, false when dropped for lost access. Raises on
    # validation failures.
    def dispatch_now!(scheduled, now: Time.current)
      dispatch_item!(scheduled, now:, immediate: true)
    end

    private
      def due_candidates(now)
        ScheduledMessage.due(now)
          .joins(:user, :room)
          .merge(User.active.without_bots)
          .merge(Room.alive)
          .where("scheduled_messages.claimed_at IS NULL OR scheduled_messages.claimed_at < ?", now - ScheduledMessage::STALE_CLAIM_AFTER)
      end

      def dispatch_item!(scheduled, now:, immediate: false)
        scope = ScheduledMessage
          .where(id: scheduled.id, sent_at: nil, dropped_at: nil)
          .where("claimed_at IS NULL OR claimed_at < ?", now - ScheduledMessage::STALE_CLAIM_AFTER)
        scope = scope.where(send_at: ..now) unless immediate
        claimed = scope.update_all(claimed_at: now, updated_at: now) == 1
        return false unless claimed

        scheduled.reload

        if !immediate && (scheduled.send_at.nil? || scheduled.send_at > now)
          # Moved after selection: release the claim so the new time fires.
          scheduled.update_columns(claimed_at: nil)
          return false
        end

        unless scheduled.sendable?
          drop!(scheduled, now:)
          return false
        end

        if scheduled.thread&.locked?
          # A locked thread is transient (moderators can unlock), so retry
          # later instead of dropping.
          scheduled.update_columns(claimed_at: nil)
          return false
        end

        post!(scheduled, now:)
        true
      rescue => error
        # A failed post must not strand the claim: clear it so the next
        # tick retries. Drop failures propagate to the caller's logging.
        begin
          ScheduledMessage.where(id: scheduled.id, sent_at: nil, dropped_at: nil).update_all(claimed_at: nil)
        rescue => claim_error
          Rails.logger.error "Scheduled claim release failed for scheduled message #{scheduled.id}: #{claim_error.class}: #{claim_error.message}"
        end
        raise error
      end

      def post!(scheduled, now:)
        message = nil

        ActiveRecord::Base.transaction do
          reply_id = scheduled.reply_to_message_id if Message.exists?(id: scheduled.reply_to_message_id)

          if scheduled.thread
            message = scheduled.thread.post_message!(
              creator: scheduled.user,
              attributes: { markdown_source: scheduled.markdown_source, reply_to_message_id: reply_id }.compact
            )
          else
            message = scheduled.room.root_messages.new(
              creator: scheduled.user,
              markdown_source: scheduled.markdown_source,
              reply_to_message_id: reply_id
            )
            message.save!
          end
          message.process_attachment
          scheduled.update!(sent_at: now, sent_message: message)
        end

        message.broadcast_create
        Message::BotWebhookFanout.deliver_for(message)
        message
      end

      def drop!(scheduled, now:, reason: nil)
        scheduled.drop!(reason: reason, now: now)
      end
  end
end
