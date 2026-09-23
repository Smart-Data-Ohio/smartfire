module BoardAutomations
  # Pushes one claimed SLA nudge to its recipient's subscriptions. Reads
  # like the event and saved-item reminder pushers: membership is
  # re-checked at delivery time, and the notification policy applies DND
  # and quiet hours (reminders carry no sender, so they stay silent).
  class NudgePusher
    attr_reader :nudge

    def initialize(nudge:)
      @nudge = nudge
    end

    def push
      return unless still_a_member?
      return unless Notifications::Policy.new(recipient: nudge.recipient, kind: :reminder).push?

      enqueue_payload_for_delivery build_payload, Push::Subscription.where(user_id: nudge.recipient_id)
    end

    private
      def build_payload
        thread = nudge.channel_thread

        {
          title: nudge.room.name,
          body: push_body(thread),
          path: Rails.application.routes.url_helpers.room_path(nudge.room, thread: thread.id)
        }
      end

      def push_body(thread)
        status = ChannelThread::WORK_STATUS_LABELS.fetch(nudge.work_status, nudge.work_status.to_s.humanize)
        prefix = nudge.stage == "escalation" ? "Escalated" : "SLA breach"

        "#{prefix}: #{thread.name} sitting in #{status}"
      end

      def still_a_member?
        nudge.recipient.active? && !nudge.recipient.bot? &&
          nudge.room.memberships.exists?(user_id: nudge.recipient_id)
      end

      def enqueue_payload_for_delivery(payload, subscriptions)
        Rails.configuration.x.web_push_pool.queue(payload, subscriptions)
      end
  end
end
