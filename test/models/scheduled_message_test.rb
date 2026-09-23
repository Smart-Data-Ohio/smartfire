require "test_helper"

class ScheduledMessageTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @room = rooms(:watercooler)
  end

  test "schedules for the future" do
    scheduled = ScheduledMessage.create!(
      user: @user, room: @room, markdown_source: "Morning!", send_at: 1.hour.from_now
    )

    assert scheduled.pending?
    assert scheduled.sendable?
  end

  test "rejects past and blank send times and bodies" do
    scheduled = ScheduledMessage.new(user: @user, room: @room, markdown_source: "Hi", send_at: 1.hour.ago)
    assert_not scheduled.valid?
    assert_includes scheduled.errors[:send_at], "must be in the future"

    scheduled = ScheduledMessage.new(user: @user, room: @room, markdown_source: "", send_at: 1.hour.from_now)
    assert_not scheduled.valid?
  end

  test "threads must belong to the room" do
    other_room = rooms(:designers)
    thread = ChannelThread.create!(room: other_room, creator: @user, name: "Design chat")

    scheduled = ScheduledMessage.new(user: @user, room: @room, thread: thread, markdown_source: "Hi", send_at: 1.hour.from_now)

    assert_not scheduled.valid?
    assert_includes scheduled.errors[:thread], "must belong to the scheduled room"
  end

  test "unusable rows are not sendable" do
    scheduled = ScheduledMessage.create!(
      user: @user, room: @room, markdown_source: "Morning!", send_at: 1.hour.from_now
    )

    @room.memberships.where(user: @user).delete_all
    assert_not scheduled.sendable?
  end

  test "deleting a sent message keeps the history row with its link cleared" do
    scheduled = ScheduledMessage.create!(
      user: @user, room: @room, markdown_source: "Morning!", send_at: 1.hour.from_now
    )
    ScheduledMessage::Dispatcher.dispatch_now!(scheduled)
    message = scheduled.reload.sent_message
    assert_not_nil message

    message.destroy!

    assert scheduled.reload.sent?
    assert_nil scheduled.sent_message_id
  end

  test "deleting a reply target keeps the pending row with its link cleared" do
    target = @room.root_messages.create!(creator: @user, markdown_source: "Target")
    scheduled = ScheduledMessage.create!(
      user: @user, room: @room, markdown_source: "Reply soon",
      reply_to_message: target, send_at: 1.hour.from_now
    )

    target.destroy!

    assert scheduled.reload.pending?
    assert_nil scheduled.reply_to_message_id
  end

  test "deleting a thread drops pending rows with an inbox item" do
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    target = thread.post_message!(creator: @user, attributes: { markdown_source: "Target" })
    scheduled = ScheduledMessage.create!(
      user: @user, room: @room, thread: thread, reply_to_message: target,
      markdown_source: "Thread soon", send_at: 1.hour.from_now
    )

    assert_difference -> { ActivityItem.where(user: @user, source: scheduled).count }, 1 do
      thread.destroy!
    end

    scheduled.reload
    assert scheduled.dropped?
    assert_equal "its thread was deleted", scheduled.drop_reason
    assert_nil scheduled.thread_id
    item = ActivityItem.where(user: @user, source: scheduled).sole
    assert_equal "scheduled_message_dropped", item.event_type
    assert_includes ActivityItem.accessible_to(@user), item
  end

  test "deleting a thread keeps sent history rows with the thread link cleared" do
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    scheduled = ScheduledMessage.create!(
      user: @user, room: @room, thread: thread,
      markdown_source: "Thread hi", send_at: 1.hour.from_now
    )
    ScheduledMessage::Dispatcher.dispatch_now!(scheduled)
    assert scheduled.reload.sent?

    thread.destroy!

    assert scheduled.reload.sent?
    assert_nil scheduled.thread_id
  end
end
