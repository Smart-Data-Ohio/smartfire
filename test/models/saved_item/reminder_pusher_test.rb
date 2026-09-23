require "test_helper"

class SavedItem::ReminderPusherTest < ActiveSupport::TestCase
  setup do
    @saved_item = SavedItem.create!(user: users(:david), message: messages(:first), remind_at: 1.hour.from_now)
  end

  test "pushes the reminder to the saver with a message link" do
    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, subscriptions|
      payload.fetch(:title) == rooms(:designers).name &&
        payload.fetch(:body).start_with?("Reminder: ") &&
        payload.fetch(:body).include?(messages(:first).plain_text_body.truncate(140)) &&
        payload.fetch(:path) == Rails.application.routes.url_helpers.room_at_message_path(rooms(:designers), messages(:first)) &&
        payload.fetch(:tag) == "saved-#{@saved_item.id}" &&
        subscriptions.map(&:user_id) == [ users(:david).id ]
    end

    SavedItem::ReminderPusher.new(saved_item: @saved_item).push
  end

  test "no push goes out after the saver loses room access" do
    memberships(:david_designers).destroy!

    Rails.configuration.x.web_push_pool.expects(:queue).never

    SavedItem::ReminderPusher.new(saved_item: @saved_item).push
  end

  test "a thread message reminder links into its thread" do
    thread = ChannelThread.create!(room: rooms(:designers), creator: users(:jason), name: "Deep dive")
    reply = thread.post_message!(creator: users(:jason), attributes: { markdown_source: "Threaded thought", client_message_id: "pusher-thread" })
    saved_item = SavedItem.create!(user: users(:david), message: reply, remind_at: 1.hour.from_now)

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:path) == Rails.application.routes.url_helpers.room_path(rooms(:designers), thread: thread.id, message_id: reply.id)
    end

    SavedItem::ReminderPusher.new(saved_item:).push
  end
end
