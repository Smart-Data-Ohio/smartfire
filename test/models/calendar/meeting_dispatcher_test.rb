require "test_helper"

class Calendar::MeetingDispatcherTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @user.update!(meeting_status_enabled: true)
  end

  test "a meeting start broadcasts the badge" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    assert_turbo_stream_broadcasts [ @user, :status ], count: 1 do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end

  test "a re-run with no flip broadcasts nothing" do
    cache = Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])
    assert cache.claim_broadcast!(true)

    assert_no_turbo_stream_broadcasts [ @user, :status ] do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end

  test "a meeting end broadcasts the badge" do
    cache = Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 55.minutes.ago.iso8601, 5.minutes.ago.iso8601 ] ])
    assert cache.claim_broadcast!(true)

    assert_turbo_stream_broadcasts [ @user, :status ], count: 1 do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end

  test "a broadcast carries the meeting label" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    streams = capture_turbo_stream_broadcasts([ @user, :status ]) do
      Calendar::MeetingDispatcher.dispatch_due!
    end

    assert_equal 1, streams.size
    assert_includes streams.first.to_html, "In a meeting"
  end

  test "a stale cache enqueues a refresh" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: 16.minutes.ago)

    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ @user.id ]) do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end

  test "a missing cache enqueues a refresh without broadcasting" do
    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ @user.id ]) do
      assert_turbo_stream_broadcasts [ @user, :status ], count: 0 do
        Calendar::MeetingDispatcher.dispatch_due!
      end
    end
  end

  test "a fresh cache enqueues nothing" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current)

    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end

  test "members who never opted in are ignored" do
    @user.update!(meeting_status_enabled: false)

    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      assert_turbo_stream_broadcasts [ @user, :status ], count: 0 do
        Calendar::MeetingDispatcher.dispatch_due!
      end
    end
  end

  test "deactivated members are ignored" do
    @user.update!(status: :deactivated)

    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end

  test "one failing member does not stop the sweep" do
    other = users(:jason)
    other.update!(meeting_status_enabled: true)
    Calendar::MeetingCache.create!(user: other, fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])
    Calendar::MeetingCache.create!(user: @user, fetched_at: 16.minutes.ago)
    Calendar::MeetingRefreshJob.stubs(:perform_later).raises(StandardError)

    assert_turbo_stream_broadcasts [ other, :status ], count: 1 do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end
end
