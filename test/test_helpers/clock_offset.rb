# frozen_string_literal: true

# Suite-wide clock shifter for hunting date-dependent tests. Enabled by
# TEST_CLOCK_OFFSET_DAYS (integer days, usually positive); 0 or unset
# disables it. Installed from test_helper.rb.
#
# The offset is applied in a prepended #before_setup so it lands before
# ActiveRecord::TestFixtures loads fixtures: fixture ERB such as
# <%= 1.hour.ago %> then evaluates under the shifted clock, exactly as if
# the suite ran on that future date. (Transactional tests load fixtures
# once per worker process, so the ERB sees the first test's shifted
# clock; later tests reuse those rows.) ActiveSupport's after_teardown
# hook calls travel_back after every test, so each test re-applies the
# offset from the real clock; offsets never accumulate across tests.
#
# Tests that pin their own clock with travel_to keep working: a block-form
# travel_to restores this offset when the block ends, and a setup-level
# travel_to without a block simply overrides it for that test.
#
# Caveats: the clock is frozen at (now + offset) for the duration of each
# test, and system tests only shift the server process — the driven
# browser's JS clock still reads real time.
module ClockOffsetTestHelper
  def before_setup
    if defined?(TEST_CLOCK_OFFSET) && TEST_CLOCK_OFFSET
      travel TEST_CLOCK_OFFSET
    end
    super
  end
end
