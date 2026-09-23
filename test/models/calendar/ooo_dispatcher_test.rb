require "test_helper"

class Calendar::OooDispatcherTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
  end

  test "a manual OOO start broadcasts the badge and the DM notice" do
    @user.update!(ooo_until: 1.hour.from_now, ooo_note: "Back soon")

    assert_turbo_stream_broadcasts [ @user, :status ], count: 1 do
      assert_turbo_stream_broadcasts [ @user, :ooo_notice ], count: 1 do
        Calendar::OooDispatcher.dispatch_due!
      end
    end
  end

  test "a re-run with no flip broadcasts nothing" do
    @user.update!(ooo_until: 1.hour.from_now)
    assert @user.claim_ooo_broadcast!(true)

    assert_no_turbo_stream_broadcasts [ @user, :status ] do
      assert_no_turbo_stream_broadcasts [ @user, :ooo_notice ] do
        Calendar::OooDispatcher.dispatch_due!
      end
    end
  end

  test "a steady-state tick issues no claim write" do
    @user.update!(ooo_until: 1.hour.from_now)
    Calendar::OooDispatcher.dispatch_due!

    assert_empty update_statements { Calendar::OooDispatcher.dispatch_due! }
  end

  test "an OOO end broadcasts and clears the manual columns" do
    @user.update!(ooo_until: 1.hour.from_now, ooo_note: "Back soon")
    @user.reload.claim_ooo_broadcast!(true)

    travel_to 2.hours.from_now do
      assert_turbo_stream_broadcasts [ @user, :status ], count: 1 do
        assert_turbo_stream_broadcasts [ @user, :ooo_notice ], count: 1 do
          Calendar::OooDispatcher.dispatch_due!
        end
      end
    end

    assert_nil @user.reload.ooo_until
    assert_nil @user.reload.ooo_note
  end

  test "the broadcasts carry the OOO label, the note, and the return date" do
    @user.update!(time_zone: "UTC", ooo_until: Time.zone.parse("2026-09-24T12:00:00Z"), ooo_note: "Back soon")

    badge = nil
    notice = nil
    badge = capture_turbo_stream_broadcasts([ @user, :status ]) do
      notice = capture_turbo_stream_broadcasts([ @user, :ooo_notice ]) do
        Calendar::OooDispatcher.dispatch_due!
      end
    end

    assert_equal 1, badge.size
    assert_includes badge.first.to_html, "Out of office"
    assert_includes badge.first.to_html, "Back soon"
    assert_equal 1, notice.size
    assert_includes notice.first.to_html, "is out of office until September 24, 2026"
    assert_includes notice.first.to_html, "Back soon"
  end

  test "a calendar OOO start broadcasts the badge and the notice" do
    @user.update!(ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      ooo_intervals: [ [ 5.minutes.ago.iso8601, 2.days.from_now.iso8601 ] ])

    assert_turbo_stream_broadcasts [ @user, :status ], count: 1 do
      assert_turbo_stream_broadcasts [ @user, :ooo_notice ], count: 1 do
        Calendar::OooDispatcher.dispatch_due!
      end
    end
  end

  test "a stale OOO-only cache enqueues a refresh" do
    @user.update!(ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: 16.minutes.ago)

    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ @user.id ]) do
      Calendar::OooDispatcher.dispatch_due!
    end
  end

  test "a member with both opt-ins refreshes through the meeting dispatcher only" do
    @user.update!(meeting_status_enabled: true, ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: 16.minutes.ago)

    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      Calendar::OooDispatcher.dispatch_due!
    end

    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ @user.id ]) do
      Calendar::MeetingDispatcher.dispatch_due!
    end
  end

  test "a missing cache enqueues a refresh without broadcasting" do
    @user.update!(ooo_calendar_enabled: true)

    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ @user.id ]) do
      assert_turbo_stream_broadcasts [ @user, :status ], count: 0 do
        assert_turbo_stream_broadcasts [ @user, :ooo_notice ], count: 0 do
          Calendar::OooDispatcher.dispatch_due!
        end
      end
    end
  end

  test "members with neither a manual OOO nor the calendar opt-in are ignored" do
    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      assert_turbo_stream_broadcasts [ @user, :status ], count: 0 do
        assert_turbo_stream_broadcasts [ @user, :ooo_notice ], count: 0 do
          Calendar::OooDispatcher.dispatch_due!
        end
      end
    end
  end

  test "deactivated members are ignored" do
    @user.update!(ooo_until: 1.hour.from_now, status: :deactivated)

    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      Calendar::OooDispatcher.dispatch_due!
    end
  end

  test "one failing member does not stop the sweep" do
    other = users(:jason)
    other.update!(ooo_until: 1.hour.from_now)
    @user.update!(ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: 16.minutes.ago)
    Calendar::MeetingRefreshJob.stubs(:perform_later).raises(StandardError)

    assert_turbo_stream_broadcasts [ other, :status ], count: 1 do
      assert_turbo_stream_broadcasts [ other, :ooo_notice ], count: 1 do
        Calendar::OooDispatcher.dispatch_due!
      end
    end
  end

  private
    # Every UPDATE statement the block issues, even a no-op one: under the
    # immediate transaction mode each still takes the database write lock.
    def update_statements(&block)
      statements = []
      subscriber = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        statements << payload[:sql] if payload[:sql].to_s.start_with?("UPDATE")
      end
      block.call
      statements
    ensure
      ActiveSupport::Notifications.unsubscribe(subscriber)
    end
end
