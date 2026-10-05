# Observable consequence of recurrence_test.rb:1048 at pinned d7c7de92.
require_relative "../../../test/test_helper"

class CutoverEventGuardResetTest < ActiveSupport::TestCase
  test "head-only scoped retime clears the guard before the next plain update" do
    travel_to Time.utc(2026, 9, 22, 12) do
      head = rooms(:designers).events.create!(organizer: users(:david), title: "Planning session",
        starts_at: Time.utc(2027, 1, 1, 9), ends_at: Time.utc(2027, 1, 1, 10), time_zone: "UTC",
        recurrence_rule: "weekly", recurrence_until: Date.new(2027, 1, 5))
      head.update_with_scope!({ starts_at: Time.utc(2027, 1, 1, 10), ends_at: Time.utc(2027, 1, 1, 11) },
        scope: "this_and_following", actor: users(:david))
      assert_not head.instance_variable_get(:@following_reorder)
      error = assert_raises(ActiveRecord::RecordInvalid) do
        head.update!(starts_at: Time.utc(2027, 1, 1, 12), ends_at: Time.utc(2027, 1, 1, 13))
      end
      assert_equal [ "moves the whole series: choose This and following or the entire series" ], error.record.errors[:starts_at]
      assert_equal Time.utc(2027, 1, 1, 10), head.reload.starts_at
    end
  end
end
