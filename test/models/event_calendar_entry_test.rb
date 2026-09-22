require "test_helper"

class EventCalendarEntryTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @event = events(:launch_party)
    @david = users(:david)
  end

  test "destroying an entry enqueues its remote delete" do
    entry = EventCalendarEntry.create!(event: @event, user: @david, google_event_id: "orphan-id")

    assert_enqueued_with(job: Calendar::RemoteDeleteJob, args: [ @david.id, "orphan-id" ]) do
      entry.destroy!
    end
  end

  test "delete skips the remote delete for already-reconciled rows" do
    entry = EventCalendarEntry.create!(event: @event, user: @david, google_event_id: "reconciled-id")

    assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do
      entry.delete
    end
  end

  test "delete_all skips remote deletes for disconnect cleanup" do
    EventCalendarEntry.create!(event: @event, user: @david, google_event_id: "disconnect-id")

    assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do
      @david.event_calendar_entries.delete_all
    end
  end
end
