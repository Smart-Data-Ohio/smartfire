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
