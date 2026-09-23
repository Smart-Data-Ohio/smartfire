class SavedItem::ReminderPusher
  attr_reader :saved_item

  def initialize(saved_item:)
    @saved_item = saved_item
  end

  def push
    return unless still_a_member?

    enqueue_payload_for_delivery build_payload, Push::Subscription.where(user_id: saved_item.user_id)
  end

  private
    def build_payload
      message = saved_item.message
      room = message.room

      {
        title: room.direct? ? message.creator.name : room.name,
        body: "Reminder: #{message.plain_text_body.truncate(140)}",
        path: message_path(message)
      }
    end

    def message_path(message)
      helpers = Rails.application.routes.url_helpers
      if message.thread_message?
        helpers.room_path(message.room, thread: message.thread_id, message_id: message.id)
      else
        helpers.room_at_message_path(message.room, message)
      end
    end

    # Membership is re-checked at delivery time, like the inbox item:
    # a saver removed between the claim and the push gets nothing.
    def still_a_member?
      message = saved_item.message
      message.room.memberships.exists?(user_id: saved_item.user_id)
    end

    def enqueue_payload_for_delivery(payload, subscriptions)
      Rails.configuration.x.web_push_pool.queue(payload, subscriptions)
    end
end
