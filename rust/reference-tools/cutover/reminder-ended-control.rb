require_relative '../../../test/test_helper'
class CutoverReminderEndedControlTest < ActiveSupport::TestCase
  test 'recently ended suppression has a still-running control' do
    travel_to Time.utc(2026, 3, 2, 16) do
      calls = []
      Rails.configuration.x.web_push_pool.stubs(:queue).with do |payload, _subscriptions|
        calls << payload
        true
      end
      [ -120, -3 ].each do |minutes|
        event = rooms(:designers).events.create!(organizer: users(:david), title: 'Missed sync',
          starts_at: minutes.minutes.from_now, ends_at: 1.minute.ago, time_zone: 'UTC')
        Event::ReminderPusher.new(event:).push
        assert_empty calls
      end
      event = rooms(:designers).events.create!(organizer: users(:david), title: 'Quick sync',
        starts_at: 4.minutes.ago, ends_at: 1.hour.from_now, time_zone: 'UTC')
      Event::ReminderPusher.new(event:).push
      assert_equal [ 'Starting now: Quick sync' ], calls.map { |payload| payload.fetch(:body) }
    end
  end
end
