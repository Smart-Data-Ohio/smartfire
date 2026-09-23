require "test_helper"

class ScheduledMessage::DispatcherTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @room = rooms(:watercooler)
  end

  test "dispatch_due posts due rows as the author" do
    scheduled = schedule!(markdown_source: "Morning!", send_at: 1.minute.from_now)
    future = schedule!(markdown_source: "Later", send_at: 2.hours.from_now)

    travel 2.minutes do
      assert_difference -> { @room.messages.count }, 1 do
        ScheduledMessage::Dispatcher.dispatch_due!
      end
    end

    message = @room.messages.ordered.last
    assert_equal "Morning!", message.plain_text_body
    assert_equal @user, message.creator
    assert scheduled.reload.sent?
    assert_equal message, scheduled.sent_message
    assert future.reload.pending?
  end

  test "each row sends exactly once across repeated runs" do
    schedule_due!(markdown_source: "Once")

    assert_difference -> { @room.messages.count }, 1 do
      ScheduledMessage::Dispatcher.dispatch_due!
      ScheduledMessage::Dispatcher.dispatch_due!
    end
  end

  test "a moved send time fires at the new time" do
    scheduled = schedule_due!(markdown_source: "Moved")
    ScheduledMessage.where(id: scheduled.id).update_all(send_at: 2.hours.from_now)

    assert_no_difference -> { @room.messages.count } do
      ScheduledMessage::Dispatcher.dispatch_due!
    end

    assert scheduled.reload.pending?
  end

  test "rows whose author lost access are dropped with an inbox item" do
    scheduled = schedule_due!(markdown_source: "Stranded")
    @room.memberships.where(user: @user).delete_all

    assert_no_difference -> { @room.messages.count } do
      assert_difference -> { ActivityItem.where(user: @user).count }, 1 do
        ScheduledMessage::Dispatcher.dispatch_due!
      end
    end

    assert scheduled.reload.dropped?
    item = ActivityItem.where(user: @user).ordered.first
    assert_equal "scheduled_message_dropped", item.event_type
    assert_equal scheduled, item.source
    # Visible despite the lost membership: that is the point of the item.
    assert_includes ActivityItem.accessible_to(@user), item
  end

  test "drop items are private to the author" do
    scheduled = schedule_due!(markdown_source: "Stranded")
    @room.memberships.where(user: @user).delete_all
    ScheduledMessage::Dispatcher.dispatch_due!

    assert_empty ActivityItem.accessible_to(users(:jason)).where(source: scheduled)
  end

  test "rows in soft-deleted rooms are dropped with an inbox item" do
    scheduled = schedule_due!(markdown_source: "Doomed room")
    @room.begin_destroy!

    assert_no_difference -> { Message.count } do
      assert_difference -> { ActivityItem.where(user: @user, source: scheduled).count }, 1 do
        ScheduledMessage::Dispatcher.dispatch_due!
      end
    end

    scheduled.reload
    assert scheduled.dropped?
    assert_equal "its room was deleted", scheduled.drop_reason
    assert_includes ActivityItem.accessible_to(@user),
      ActivityItem.where(user: @user, source: scheduled).sole
  end

  test "thread rows post inside the thread" do
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    schedule_due!(markdown_source: "Thread hi", thread: thread)

    assert_difference -> { thread.messages.count }, 1 do
      ScheduledMessage::Dispatcher.dispatch_due!
    end
  end

  test "thread rows post like thread replies: no legacy fanout, one attachment pass" do
    legacy = User.create_bot!(name: "Legacy Note", webhook_url: "https://example.test/legacy-note")
    @room.memberships.grant_to(legacy)
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    schedule_due!(markdown_source: "Hey @[Legacy Note]", thread: thread)
    Message.any_instance.expects(:process_attachment).once

    assert_no_enqueued_jobs only: Bot::WebhookJob do
      assert_difference -> { thread.messages.count }, 1 do
        ScheduledMessage::Dispatcher.dispatch_due!
      end
    end
  end

  test "locked threads retry instead of dropping" do
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    scheduled = schedule_due!(markdown_source: "Thread hi", thread: thread)
    thread.update!(locked_at: Time.current)

    assert_no_difference -> { thread.messages.count } do
      ScheduledMessage::Dispatcher.dispatch_due!
    end

    assert scheduled.reload.pending?
    assert_not scheduled.dropped?
  end

  test "a send-time validation failure drops the row with the reason" do
    board = Rooms::Board.create_for({ name: "Launch", creator: @user }, users: [ @user ])
    scheduled = ScheduledMessage.create!(user: @user, room: board, markdown_source: "Root post", send_at: 1.hour.from_now)
    scheduled.update_columns(send_at: 1.minute.ago)

    assert_no_difference -> { Message.count } do
      assert_difference -> { ActivityItem.where(user: @user, source: scheduled).count }, 1 do
        ScheduledMessage::Dispatcher.dispatch_due!
      end
    end

    scheduled.reload
    assert scheduled.dropped?
    assert_match "board", scheduled.drop_reason

    # Never retries: a later tick posts nothing and changes nothing.
    assert_no_difference -> { Message.count } do
      assert_no_difference -> { ActivityItem.where(user: @user, source: scheduled).count } do
        ScheduledMessage::Dispatcher.dispatch_due!
      end
    end
    assert scheduled.reload.dropped?
  end

  test "a row dropped after the claim posts nothing" do
    thread = ChannelThread.create!(room: @room, creator: @user, name: "Side chat")
    scheduled = schedule_due!(markdown_source: "Thread hi", thread: thread)
    # The claim landed, then the thread was destroyed before the post:
    # the destroy callback dropped the row and nullified thread_id.
    ScheduledMessage.where(id: scheduled.id).update_all(claimed_at: Time.current)
    thread.destroy!
    assert scheduled.reload.dropped?

    posted = nil
    assert_no_difference -> { Message.count } do
      posted = ScheduledMessage::Dispatcher.send(:post!, scheduled, now: Time.current)
    end

    assert_nil posted
  end

  test "dispatch_now sends immediately and reports drops" do
    scheduled = schedule!(markdown_source: "Now", send_at: 2.hours.from_now)

    assert_difference -> { @room.messages.count }, 1 do
      assert ScheduledMessage::Dispatcher.dispatch_now!(scheduled)
    end

    stranded = schedule!(markdown_source: "Stranded", send_at: 2.hours.from_now)
    @room.memberships.where(user: @user).delete_all

    assert_not ScheduledMessage::Dispatcher.dispatch_now!(stranded)
    assert stranded.reload.dropped?
  end

  private
    def schedule!(**attributes)
      ScheduledMessage.create!(user: @user, room: @room, **attributes)
    end

    # A row whose time already passed: created valid, then backdated.
    def schedule_due!(**attributes)
      scheduled = schedule!(send_at: 1.hour.from_now, **attributes)
      scheduled.update_columns(send_at: 1.minute.ago)
      scheduled
    end
end
