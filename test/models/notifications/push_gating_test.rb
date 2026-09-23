require "test_helper"

class Notifications::PushGatingTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @pool = Rails.configuration.x.web_push_pool
  end

  test "room push skips a DND recipient but the inbox item is still recorded" do
    users(:jason).update!(dnd_enabled: true)
    message = @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:jason)}", client_message_id: "gating-dnd"
    )

    @pool.expects(:queue).once.with do |_payload, subscriptions|
      assert_not_includes subscriptions.map(&:user_id), users(:jason).id
      true
    end

    Room::MessagePusher.new(room: @room, message:).push
    assert_equal "mention", ActivityItem.find_by!(user: users(:jason), source: message).event_type
  end

  test "room push still reaches a starred sender's recipient during DND" do
    users(:jason).update!(dnd_enabled: true)
    DndAllowedUser.create!(user: users(:jason), allowed_user: users(:david))
    message = @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:jason)}", client_message_id: "gating-starred"
    )

    @pool.expects(:queue).once.with do |_payload, subscriptions|
      assert_includes subscriptions.map(&:user_id), users(:jason).id
      true
    end

    Room::MessagePusher.new(room: @room, message:).push
  end

  test "room push skips a recipient inside quiet hours" do
    travel_to Time.zone.parse("2026-09-23 12:00") do
      users(:jason).update!(quiet_hours_enabled: true, quiet_hours_start: "09:00", quiet_hours_end: "17:00")
      message = @room.messages.create!(
        creator: users(:david), body: "Hey #{mention_attachment_for(:jason)}", client_message_id: "gating-quiet"
      )

      @pool.expects(:queue).once.with do |_payload, subscriptions|
        assert_not_includes subscriptions.map(&:user_id), users(:jason).id
        true
      end

      Room::MessagePusher.new(room: @room, message:).push
    end
  end

  test "thread push notifies followers with the thread payload" do
    thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "Gated thread")
    ThreadMembership.join!(thread, users(:jz))
    ThreadMembership.join!(thread, users(:jason)).update!(involvement: "everything")
    message = thread.post_message!(creator: users(:jz),
      attributes: { markdown_source: "Open here", client_message_id: "gating-thread" })

    @pool.expects(:queue).once.with do |payload, subscriptions|
      assert_equal [ users(:jason).id ], subscriptions.order(:id).pluck(:user_id)
      assert_equal thread.name, payload[:title]
      true
    end

    ChannelThread::MessagePusher.new(thread:, message:).push
  end

  test "thread push skips a DND follower" do
    users(:jason).update!(dnd_enabled: true)
    thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "DND thread")
    ThreadMembership.join!(thread, users(:jz))
    ThreadMembership.join!(thread, users(:jason)).update!(involvement: "everything")
    message = thread.post_message!(creator: users(:jz),
      attributes: { markdown_source: "Quiet update", client_message_id: "gating-thread-dnd" })

    @pool.expects(:queue).never

    ChannelThread::MessagePusher.new(thread:, message:).push
    assert_equal "thread_activity", ActivityItem.find_by!(user: users(:jason), source: message).event_type
  end

  test "a thread reply pushes its follower author but not an unfollowed one" do
    thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "Reply gating thread")
    ThreadMembership.join!(thread, users(:jz)).update!(involvement: "everything")
    ThreadMembership.join!(thread, users(:jason)).update!(involvement: "mentions")

    unfollowed_source = thread.post_message!(creator: users(:jason),
      attributes: { markdown_source: "Original", client_message_id: "gating-reply-source" })
    unfollowed_reply = thread.post_message!(creator: users(:jz),
      attributes: { markdown_source: "Answer", reply_to_message: unfollowed_source, client_message_id: "gating-reply" })

    @pool.expects(:queue).never
    ChannelThread::MessagePusher.new(thread:, message: unfollowed_reply).push
    assert_not ActivityItem.exists?(user: users(:jason), source: unfollowed_reply)

    ThreadMembership.find_by!(thread:, user: users(:jason)).update!(involvement: "everything")
    followed_source = thread.post_message!(creator: users(:jason),
      attributes: { markdown_source: "Original 2", client_message_id: "gating-reply-source-2" })
    followed_reply = thread.post_message!(creator: users(:jz),
      attributes: { markdown_source: "Answer 2", reply_to_message: followed_source, client_message_id: "gating-reply-2" })

    @pool.expects(:queue).once.with do |payload, subscriptions|
      assert_equal [ users(:jason).id ], subscriptions.order(:id).pluck(:user_id)
      assert_equal "Reply in #{thread.name}", payload[:title]
      true
    end

    ChannelThread::MessagePusher.new(thread:, message: followed_reply).push
    assert_equal "reply", ActivityItem.find_by!(user: users(:jason), source: followed_reply).event_type
  end

  test "reminder push skips a DND attendee" do
    users(:david).update!(dnd_enabled: true)
    event = events(:launch_party)

    @pool.expects(:queue).once.with do |_payload, subscriptions|
      assert_not_includes subscriptions.map(&:user_id), users(:david).id
      true
    end

    Event::ReminderPusher.new(event:).push
  end

  test "huddle push honors DND with a starred-caller exception" do
    original_secret = ENV["LIVEKIT_API_SECRET"]
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    item = ActivityItem.find_by!(user: users(:jason), source: grant)

    users(:jason).update!(dnd_enabled: true)
    @pool.expects(:queue).never
    Huddle::PushInvitationJob.perform_now(item.id)

    DndAllowedUser.create!(user: users(:jason), allowed_user: users(:david))
    @pool.expects(:queue).once
    Huddle::PushInvitationJob.perform_now(item.id)
  ensure
    ENV["LIVEKIT_API_SECRET"] = original_secret
  end
end
