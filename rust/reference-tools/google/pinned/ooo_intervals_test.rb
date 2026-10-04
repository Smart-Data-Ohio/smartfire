require "test_helper"

class Calendar::OooIntervalsTest < ActiveSupport::TestCase
  test "derives intervals from timed out-of-office events" do
    items = [
      ooo_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z"),
      ooo_event("2026-09-24T10:00:00Z", "2026-09-24T11:00:00Z")
    ]

    assert_equal [
      [ Time.zone.parse("2026-09-23T10:00:00Z"), Time.zone.parse("2026-09-23T11:00:00Z") ],
      [ Time.zone.parse("2026-09-24T10:00:00Z"), Time.zone.parse("2026-09-24T11:00:00Z") ]
    ], Calendar::OooIntervals.from_items(items, zone: "UTC")
  end

  test "ordinary events never count, even timed ones" do
    items = [
      timed_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z"),
      timed_event("2026-09-23T13:00:00Z", "2026-09-23T13:30:00Z", "eventType" => "default")
    ]

    assert_empty Calendar::OooIntervals.from_items(items, zone: "UTC")
  end

  test "cancelled out-of-office events never count" do
    items = [ ooo_event("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "status" => "cancelled") ]

    assert_empty Calendar::OooIntervals.from_items(items, zone: "UTC")
  end

  test "all-day out-of-office events resolve in the member's zone" do
    items = [
      { "eventType" => "outOfOffice", "status" => "confirmed",
        "start" => { "date" => "2026-09-28" },
        "end" => { "date" => "2026-10-05" } }
    ]

    zone = ActiveSupport::TimeZone["Pacific Time (US & Canada)"]
    assert_equal [ [ zone.parse("2026-09-28"), zone.parse("2026-10-05") ] ],
      Calendar::OooIntervals.from_items(items, zone: "Pacific Time (US & Canada)")
  end

  test "malformed items are skipped without raising" do
    items = [
      nil,
      "not-an-event",
      {},
      ooo_event("not-a-time", "2026-09-23T11:00:00Z"),
      ooo_event("2026-09-23T11:00:00Z", "2026-09-23T10:00:00Z"),
      ooo_event("2026-09-23T10:00:00Z", "2026-09-23T10:00:00Z"),
      { "eventType" => "outOfOffice", "status" => "confirmed",
        "start" => { "date" => "not-a-date" }, "end" => { "date" => "2026-10-05" } }
    ]

    assert_empty Calendar::OooIntervals.from_items(items, zone: "UTC")
  end

  private
    def ooo_event(start_at, end_at, **attrs)
      timed_event(start_at, end_at, "eventType" => "outOfOffice", **attrs)
    end

    def timed_event(start_at, end_at, **attrs)
      {
        "status" => "confirmed",
        "start" => { "dateTime" => start_at, "timeZone" => "UTC" },
        "end" => { "dateTime" => end_at, "timeZone" => "UTC" }
      }.merge(attrs)
    end
end
