require "test_helper"

# Covers the TEST_CLOCK_OFFSET_DAYS suite clock shifter itself: with the
# variable set, every test's clock reads the shifted time and fixture ERB
# evaluated under it. Skipped in normal runs; exercised by the offset runs
# in docs/development.md.
class ClockOffsetTest < ActiveSupport::TestCase
  test "the offset shifts Time.current and fixture ERB times" do
    skip "set TEST_CLOCK_OFFSET_DAYS to exercise the suite clock shifter" if TEST_CLOCK_OFFSET.nil?

    real_now = travel_back { Time.now }
    assert_in_delta (real_now + TEST_CLOCK_OFFSET).to_f, Time.current.to_f, 60,
      "Time.current must read the shifted clock"
    assert_in_delta (real_now + TEST_CLOCK_OFFSET - 1.hour).to_f, messages(:first).created_at.to_f, 30.minutes.to_i,
      "fixture ERB must evaluate under the shifted clock"
  end
end
