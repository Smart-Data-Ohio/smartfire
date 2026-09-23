require "test_helper"

class Calendar::MeetingCacheTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
  end

  test "in_meeting? is true inside an interval, with an inclusive start and exclusive end" do
    cache = Calendar::MeetingCache.create!(user: @user,
      busy_intervals: [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ])

    assert_not cache.in_meeting?(now: Time.zone.parse("2026-09-23T09:59:59Z"))
    assert cache.in_meeting?(now: Time.zone.parse("2026-09-23T10:00:00Z"))
    assert cache.in_meeting?(now: Time.zone.parse("2026-09-23T10:30:00Z"))
    assert_not cache.in_meeting?(now: Time.zone.parse("2026-09-23T11:00:00Z"))
  end

  test "in_meeting? is false without intervals" do
    cache = Calendar::MeetingCache.create!(user: @user, busy_intervals: [])

    assert_not cache.in_meeting?
  end

  test "in_meeting? ignores malformed pairs instead of raising" do
    cache = Calendar::MeetingCache.create!(user: @user,
      busy_intervals: [ [ "not-a-time", "2026-09-23T11:00:00Z" ], nil, "nope" ])

    assert_not cache.in_meeting?(now: Time.zone.parse("2026-09-23T10:30:00Z"))
  end

  test "quiet_window_epochs returns epoch windows and skips malformed pairs" do
    cache = Calendar::MeetingCache.create!(user: @user,
      busy_intervals: [
        [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ],
        nil, "nope", [ "not-a-time", "2026-09-23T11:00:00Z" ]
      ])

    assert_equal [
      [ Time.zone.parse("2026-09-23T10:00:00Z").to_i, Time.zone.parse("2026-09-23T11:00:00Z").to_i ]
    ], cache.quiet_window_epochs
  end

  test "in_ooo? is true inside an OOO interval, with an inclusive start and exclusive end" do
    cache = Calendar::MeetingCache.create!(user: @user,
      ooo_intervals: [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ])

    assert_not cache.in_ooo?(now: Time.zone.parse("2026-09-23T09:59:59Z"))
    assert cache.in_ooo?(now: Time.zone.parse("2026-09-23T10:00:00Z"))
    assert cache.in_ooo?(now: Time.zone.parse("2026-09-23T10:30:00Z"))
    assert_not cache.in_ooo?(now: Time.zone.parse("2026-09-23T11:00:00Z"))
  end

  test "in_ooo? reads only the OOO intervals, not the busy ones" do
    cache = Calendar::MeetingCache.create!(user: @user,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    assert cache.in_meeting?
    assert_not cache.in_ooo?
  end

  test "ooo_end_covering returns the latest covering end" do
    cache = Calendar::MeetingCache.create!(user: @user,
      ooo_intervals: [
        [ 2.hours.ago.iso8601, 1.hour.from_now.iso8601 ],
        [ 30.minutes.ago.iso8601, 3.hours.from_now.iso8601 ],
        [ 5.hours.ago.iso8601, 4.hours.ago.iso8601 ]
      ])

    assert_in_delta 3.hours.from_now.to_f, cache.ooo_end_covering.to_f, 1
  end

  test "ooo_end_covering is nil while uncovered" do
    cache = Calendar::MeetingCache.create!(user: @user,
      ooo_intervals: [ [ 2.hours.ago.iso8601, 1.hour.ago.iso8601 ] ])

    assert_nil cache.ooo_end_covering
  end

  test "ooo_window_epochs returns epoch windows and skips malformed pairs" do
    cache = Calendar::MeetingCache.create!(user: @user,
      ooo_intervals: [
        [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ],
        nil, "nope", [ "not-a-time", "2026-09-23T11:00:00Z" ]
      ])

    assert_equal [
      [ Time.zone.parse("2026-09-23T10:00:00Z").to_i, Time.zone.parse("2026-09-23T11:00:00Z").to_i ]
    ], cache.ooo_window_epochs
  end

  test "claim_broadcast! wins the first claim and each flip, and loses re-runs" do
    cache = Calendar::MeetingCache.create!(user: @user)

    assert cache.claim_broadcast!(true)
    assert_not cache.claim_broadcast!(true)
    assert cache.claim_broadcast!(false)
    assert_not cache.claim_broadcast!(false)
  end

  test "claim_broadcast! lets only one concurrent claimant win" do
    cache = Calendar::MeetingCache.create!(user: @user)
    first = Calendar::MeetingCache.find(cache.id)
    second = Calendar::MeetingCache.find(cache.id)

    assert first.claim_broadcast!(true)
    assert_not second.claim_broadcast!(true)
  end

  test "one cache per user" do
    Calendar::MeetingCache.create!(user: @user)

    assert_raises(ActiveRecord::RecordInvalid) do
      Calendar::MeetingCache.create!(user: @user)
    end
  end
end
