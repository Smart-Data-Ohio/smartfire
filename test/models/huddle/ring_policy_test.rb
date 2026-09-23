require "test_helper"

class Huddle::RingPolicyTest < ActiveSupport::TestCase
  teardown do
    Huddle::RingPolicy.quiet_check = nil
  end

  test "an invitation rings a member who is not in do-not-disturb" do
    assert Huddle::RingPolicy.ring?(users(:jason), caller: users(:david))
  end

  test "do-not-disturb silences the ring" do
    users(:jason).update!(presence_setting: "dnd")

    assert_not Huddle::RingPolicy.ring?(users(:jason), caller: users(:david))
  end

  test "a caller allowed during do-not-disturb still rings" do
    users(:jason).update!(presence_setting: "dnd")
    DndAllowedUser.create!(user: users(:jason), allowed_user: users(:david))

    assert Huddle::RingPolicy.ring?(users(:jason), caller: users(:david))
    assert_not Huddle::RingPolicy.ring?(users(:jason), caller: users(:kevin))
  end

  test "a quiet check override replaces the policy" do
    Huddle::RingPolicy.quiet_check = ->(user) { user == users(:jason) }

    assert_not Huddle::RingPolicy.ring?(users(:jason))
    assert Huddle::RingPolicy.ring?(users(:david))
  end

  test "quiet-during-meetings silences the ring during a busy interval" do
    users(:jason).update!(meeting_status_enabled: true, meeting_dnd_enabled: true)
    Calendar::MeetingCache.create!(user: users(:jason), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    assert_not Huddle::RingPolicy.ring?(users(:jason).reload, caller: users(:david))
  end

  test "a caller allowed during do-not-disturb still rings through a meeting" do
    users(:jason).update!(meeting_status_enabled: true, meeting_dnd_enabled: true)
    Calendar::MeetingCache.create!(user: users(:jason), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])
    DndAllowedUser.create!(user: users(:jason), allowed_user: users(:david))

    assert Huddle::RingPolicy.ring?(users(:jason).reload, caller: users(:david))
    assert_not Huddle::RingPolicy.ring?(users(:jason).reload, caller: users(:kevin))
  end
end
