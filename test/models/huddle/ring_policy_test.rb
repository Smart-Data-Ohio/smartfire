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
end
