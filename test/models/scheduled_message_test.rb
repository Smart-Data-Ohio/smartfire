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
end
