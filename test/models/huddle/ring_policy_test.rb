require "test_helper"

class Huddle::RingPolicyTest < ActiveSupport::TestCase
  teardown do
    Huddle::RingPolicy.quiet_check = nil
  end

  test "every invitation rings until quiet hours are wired in" do
    assert Huddle::RingPolicy.ring?(users(:jason))
  end

  test "a wired quiet check silences the ring without touching the banner" do
    Huddle::RingPolicy.quiet_check = ->(user) { user == users(:jason) }

    assert_not Huddle::RingPolicy.ring?(users(:jason))
    assert Huddle::RingPolicy.ring?(users(:david))
  end
end
