require "test_helper"

class Notifications::PolicyTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @recipient = users(:david)
    @sender = users(:jason)
    @thread = ChannelThread.create!(room: @room, creator: @sender, name: "Policy thread")
  end

  # Room messages: inbox

  test "a room mention records and pushes for a mentions member" do
    policy = room_policy(room_involvement: "mentions", mentioned: true)

    assert_equal "mention", policy.inbox_event_type
    assert policy.push?
    assert policy.sound?
  end

  test "a room mention still records with notifications off but sends no push" do
    policy = room_policy(room_involvement: "nothing", mentioned: true)

    assert_equal "mention", policy.inbox_event_type
    assert_not policy.push?
  end

  test "an invisible room membership gets nothing at all" do
    policy = room_policy(room_involvement: "invisible", mentioned: true,
      reply_to_recipient: true, keyword_matched: true)

    assert_nil policy.inbox_event_type
    assert_not policy.push?
  end

  test "a room reply records and pushes for mentions and everything members" do
    %w[ mentions everything ].each do |involvement|
      policy = room_policy(room_involvement: involvement, reply_to_recipient: true)

      assert_equal "reply", policy.inbox_event_type, involvement
      assert policy.push?, involvement
    end
  end

  test "a room reply stays silent with notifications off" do
    policy = room_policy(room_involvement: "nothing", reply_to_recipient: true)

    assert_nil policy.inbox_event_type
    assert_not policy.push?
  end

  test "a plain room message pushes everything followers without an inbox item" do
    policy = room_policy(room_involvement: "everything")

    assert_nil policy.inbox_event_type
    assert policy.push?
  end

  test "a plain room message does nothing for mentions members" do
    policy = room_policy(room_involvement: "mentions")

    assert_nil policy.inbox_event_type
    assert_not policy.push?
  end

  test "a room keyword match records without pushing for mentions and notifications-off members" do
    %w[ mentions nothing ].each do |involvement|
      policy = room_policy(room_involvement: involvement, keyword_matched: true)

      assert_equal "keyword_alert", policy.inbox_event_type, involvement
      assert_not policy.push?, involvement
    end
  end

  test "an everything member's keyword match still pushes as a broadcast" do
    policy = room_policy(room_involvement: "everything", keyword_matched: true)

    assert_equal "keyword_alert", policy.inbox_event_type
    assert policy.push?
  end

  test "a room message without a membership records nothing, not even mentions or keywords" do
    policy = Notifications::Policy.new(
      recipient: @recipient, sender: @sender, kind: :room_message,
      room_membership: nil, mentioned: true, reply_to_recipient: true, keyword_matched: true
    )

    assert_nil policy.inbox_event_type
  end

  test "mention beats reply beats keyword for one room message" do
    assert_equal "mention", room_policy(room_involvement: "mentions",
      mentioned: true, reply_to_recipient: true, keyword_matched: true).inbox_event_type
    assert_equal "reply", room_policy(room_involvement: "mentions",
      reply_to_recipient: true, keyword_matched: true).inbox_event_type
  end

  # Thread messages: inbox and push

  test "a followed thread records activity and pushes" do
    policy = thread_policy(thread_involvement: "everything")

    assert_equal "thread_activity", policy.inbox_event_type
    assert policy.push?
  end

  test "an unfollowed thread stays silent for plain messages" do
    policy = thread_policy(thread_involvement: "mentions")

    assert_nil policy.inbox_event_type
    assert_not policy.push?
  end

  test "a thread mention records and pushes for mentions and everything members" do
    %w[ mentions everything ].each do |involvement|
      policy = thread_policy(thread_involvement: involvement, mentioned: true)

      assert_equal "mention", policy.inbox_event_type, involvement
      assert policy.push?, involvement
    end
  end

  test "a muted thread gets nothing, not even mentions or keywords" do
    policy = thread_policy(thread_involvement: "nothing", mentioned: true,
      reply_to_recipient: true, keyword_matched: true)

    assert_nil policy.inbox_event_type
    assert_not policy.push?
  end

  test "a thread reply records and pushes for followers only" do
    follower = thread_policy(thread_involvement: "everything", reply_to_recipient: true)
    assert_equal "reply", follower.inbox_event_type
    assert follower.push?

    unfollowed = thread_policy(thread_involvement: "mentions", reply_to_recipient: true)
    assert_nil unfollowed.inbox_event_type
    assert_not unfollowed.push?
  end

  test "a thread keyword match records for unfollowed members without pushing" do
    policy = thread_policy(thread_involvement: "mentions", keyword_matched: true)

    assert_equal "keyword_alert", policy.inbox_event_type
    assert_not policy.push?
  end

  test "room notifications off suppresses thread activity but not keywords" do
    activity = thread_policy(room_involvement: "nothing", thread_involvement: "everything")
    assert_nil activity.inbox_event_type
    assert_not activity.push?

    keyword = thread_policy(room_involvement: "nothing", thread_involvement: "everything", keyword_matched: true)
    assert_equal "keyword_alert", keyword.inbox_event_type
  end

  test "a non-member of the thread gets nothing" do
    policy = thread_policy(thread_involvement: nil, mentioned: true)

    assert_nil policy.inbox_event_type
    assert_not policy.push?
  end

  test "bots and deactivated recipients record nothing" do
    assert_nil room_policy(room_involvement: "mentions", mentioned: true, recipient: users(:bender)).inbox_event_type

    @recipient.update!(status: :deactivated)
    assert_nil room_policy(room_involvement: "mentions", mentioned: true).inbox_event_type
  end

  # Do Not Disturb and quiet hours: push and sound only, never inbox

  test "manual DND suppresses push and sound but still records the inbox item" do
    @recipient.update!(dnd_enabled: true)
    policy = room_policy(room_involvement: "mentions", mentioned: true)

    assert_equal "mention", policy.inbox_event_type
    assert_not policy.push?
    assert_not policy.sound?
  end

  test "a starred sender still pushes through DND" do
    @recipient.update!(dnd_enabled: true)
    DndAllowedUser.create!(user: @recipient, allowed_user: @sender)

    assert room_policy(room_involvement: "mentions", mentioned: true).push?

    stranger = room_policy(room_involvement: "mentions", mentioned: true, sender: users(:kevin))
    assert_not stranger.push?
  end

  test "a preloaded DND exception decides without another lookup" do
    @recipient.update!(dnd_enabled: true)

    assert room_policy(room_involvement: "everything", dnd_exception: true).push?
    assert_not room_policy(room_involvement: "everything", dnd_exception: false).push?
  end

  test "quiet hours suppress push inside the window only" do
    @recipient.update!(quiet_hours_enabled: true, quiet_hours_start: "22:00", quiet_hours_end: "07:00")

    travel_to Time.zone.parse("2026-09-22 23:30") do
      assert_not room_policy(room_involvement: "everything").push?
    end
    travel_to Time.zone.parse("2026-09-23 06:30") do
      assert_not room_policy(room_involvement: "everything").push?
    end
    travel_to Time.zone.parse("2026-09-23 12:00") do
      assert room_policy(room_involvement: "everything").push?
    end
  end

  test "quiet hours follow the recipient's time zone" do
    @recipient.update!(time_zone: "Pacific Time (US & Canada)",
      quiet_hours_enabled: true, quiet_hours_start: "22:00", quiet_hours_end: "07:00")

    # 06:30 UTC is 23:30 PDT the previous day: inside the window.
    travel_to Time.zone.parse("2026-09-23 06:30") do
      assert_not room_policy(room_involvement: "everything").push?
    end
    # 16:00 UTC is 09:00 PDT: outside the window.
    travel_to Time.zone.parse("2026-09-23 16:00") do
      assert room_policy(room_involvement: "everything").push?
    end
  end

  test "a starred sender still pushes through quiet hours" do
    @recipient.update!(quiet_hours_enabled: true, quiet_hours_start: "09:00", quiet_hours_end: "17:00")
    DndAllowedUser.create!(user: @recipient, allowed_user: @sender)

    travel_to Time.zone.parse("2026-09-23 12:00") do
      assert room_policy(room_involvement: "everything").push?
    end
  end

  test "reminders push unless DND is on, and carry no sender exception" do
    assert reminder_policy.push?

    @recipient.update!(dnd_enabled: true)
    assert_not reminder_policy.push?
  end

  test "huddle invitations push unless DND is on without a starred caller" do
    assert huddle_policy.push?

    @recipient.update!(dnd_enabled: true)
    assert_not huddle_policy.push?

    DndAllowedUser.create!(user: @recipient, allowed_user: @sender)
    assert huddle_policy.push?
  end

  test "a missing recipient pushes nothing" do
    policy = Notifications::Policy.new(recipient: nil, kind: :reminder)

    assert_not policy.push?
    assert_nil policy.inbox_event_type
  end

  test "an unknown kind raises" do
    assert_raises(ArgumentError) do
      Notifications::Policy.new(recipient: @recipient, kind: :smoke_signal)
    end
  end

  test "dnd exceptions load for a batch in one query" do
    DndAllowedUser.create!(user: @recipient, allowed_user: @sender)
    kevin_id = users(:kevin).id

    ids = nil
    assert_queries_count(1) do
      ids = Notifications::Policy.dnd_exceptions_for([ @recipient.id, kevin_id ], @sender)
    end

    assert_equal Set[@recipient.id], ids
    assert_equal Set.new, Notifications::Policy.dnd_exceptions_for([ @recipient.id ], nil)
  end

  private
    def room_policy(room_involvement:, recipient: @recipient, sender: @sender, **flags)
      Notifications::Policy.new(
        recipient:, sender:, kind: :room_message,
        room_membership: Membership.new(room: @room, user: recipient, involvement: room_involvement),
        **flags
      )
    end

    def thread_policy(room_involvement: "everything", thread_involvement: "everything", recipient: @recipient, **flags)
      thread_membership = thread_involvement &&
        ThreadMembership.new(thread: @thread, user: recipient, involvement: thread_involvement)

      Notifications::Policy.new(
        recipient:, sender: @sender, kind: :thread_message,
        room_membership: Membership.new(room: @room, user: recipient, involvement: room_involvement),
        thread_membership:,
        **flags
      )
    end

    def reminder_policy
      Notifications::Policy.new(recipient: @recipient, kind: :reminder)
    end

    def huddle_policy
      Notifications::Policy.new(recipient: @recipient, sender: @sender, kind: :huddle)
    end
end
