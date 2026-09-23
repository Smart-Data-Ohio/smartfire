require "test_helper"

class Calendar::MeetingIntervalsTest < ActiveSupport::TestCase
  test "derives busy intervals from timed events" do
    items = [
      timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z"),
      timed_event("2026-09-23T13:00:00Z", "2026-09-23T13:30:00Z")
    ]

    assert_equal [
      [ Time.zone.parse("2026-09-23T10:00:00Z"), Time.zone.parse("2026-09-23T11:00:00Z") ],
      [ Time.zone.parse("2026-09-23T13:00:00Z"), Time.zone.parse("2026-09-23T13:30:00Z") ]
    ], Calendar::MeetingIntervals.from_items(items)
  end

  test "cancelled events never count" do
    items = [ timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "status" => "cancelled") ]

    assert_empty Calendar::MeetingIntervals.from_items(items)
  end

  test "out-of-office events never count as busy" do
    items = [ timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "eventType" => "outOfOffice") ]

    assert_empty Calendar::MeetingIntervals.from_items(items)
  end

  test "focus-time events never count as busy" do
    items = [ timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "eventType" => "focusTime") ]

    assert_empty Calendar::MeetingIntervals.from_items(items)
  end

  test "events declined by the member never count" do
    items = [
      timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z",
        "attendees" => [
          { "email" => "david@gmail.test", "self" => true, "responseStatus" => "declined" },
          { "email" => "jason@gmail.test", "responseStatus" => "accepted" }
        ])
    ]

    assert_empty Calendar::MeetingIntervals.from_items(items)
  end

  test "transparent (show-as-free) events never count" do
    items = [ timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "transparency" => "transparent") ]

    assert_empty Calendar::MeetingIntervals.from_items(items)
  end

  test "all-day events never count" do
    items = [
      { "status" => "confirmed",
        "start" => { "date" => "2026-09-23" },
        "end" => { "date" => "2026-09-24" } }
    ]

    assert_empty Calendar::MeetingIntervals.from_items(items)
  end

  test "tentative and needs-action events count as busy" do
    items = [
      timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "status" => "tentative"),
      timed_event("2026-09-23T13:00:00Z", "2026-09-23T13:30:00Z",
        "attendees" => [ { "self" => true, "responseStatus" => "needsAction" } ])
    ]

    assert_equal 2, Calendar::MeetingIntervals.from_items(items).size
  end

  test "organizer-only events without attendees count as busy" do
    items = [ timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z") ]

    assert_equal 1, Calendar::MeetingIntervals.from_items(items).size
  end

  test "malformed items are skipped without raising" do
    items = [
      nil,
      "not-an-event",
      {},
      timed_event("not-a-time", "2026-09-23T11:00:00Z"),
      timed_event("2026-09-23T11:00:00Z", "2026-09-23T10:00:00Z"),
      timed_event("2026-09-23T10:00:00Z", "2026-09-23T10:00:00Z")
    ]

    assert_empty Calendar::MeetingIntervals.from_items(items)
  end

  test "a declined event from another attendee still counts" do
    items = [
      timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z",
        "attendees" => [
          { "email" => "jason@gmail.test", "responseStatus" => "declined" },
          { "self" => true, "responseStatus" => "accepted" }
        ])
    ]

    assert_equal 1, Calendar::MeetingIntervals.from_items(items).size
  end

  private
    def timed_event(start_at, end_at, **attrs)
      {
        "status" => "confirmed",
        "start" => { "dateTime" => start_at, "timeZone" => "UTC" },
        "end" => { "dateTime" => end_at, "timeZone" => "UTC" }
      }.merge(attrs)
    end
end
