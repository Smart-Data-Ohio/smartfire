require "test_helper"

class User::OutOfOfficeTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @user.update!(time_zone: "UTC")
  end

  test "out of office defaults off" do
    assert_nil @user.ooo_until
    assert_nil @user.ooo_note
    assert_not @user.ooo_calendar_enabled?
    assert_not @user.ooo_notify_enabled?
    assert_not @user.manual_ooo_active?
    assert_not @user.out_of_office?
    assert_nil @user.ooo_status_text
    assert_nil @user.status_text_display
  end

  test "a manual OOO is active until its end, then reads as off" do
    @user.update!(ooo_until: 1.hour.from_now)

    assert @user.manual_ooo_active?
    assert @user.out_of_office?

    travel_to 2.hours.from_now do
      assert_not @user.manual_ooo_active?
      assert_not @user.out_of_office?
      assert_nil @user.ooo_status_text
    end
  end

  test "setting an OOO end in the past is invalid, but an expired end left behind still saves" do
    assert_not @user.update(ooo_until: 1.hour.ago)
    assert_includes @user.errors[:ooo_until], "must be in the future"

    @user.update_columns(ooo_until: 1.hour.ago)
    assert @user.update(presence_setting: "dnd")
  end

  test "a note longer than 140 characters is invalid" do
    assert_not @user.update(ooo_until: 1.day.from_now, ooo_note: "x" * 141)
    assert @user.update(ooo_until: 1.day.from_now, ooo_note: "x" * 140)
  end

  test "the status line names the return date and the note" do
    travel_to Time.zone.parse("2026-09-23 12:00") do
      @user.update!(ooo_until: Time.zone.parse("2026-09-24").end_of_day, ooo_note: "Back soon")

      assert_equal "🌴 Out of office until September 24, 2026 — Back soon", @user.ooo_status_text
      assert_equal "🌴 Out of office until September 24, 2026 — Back soon", @user.status_text_display
    end
  end

  test "the return date renders in the OOO member's own zone" do
    @user.update!(time_zone: "Pacific Time (US & Canada)", ooo_until: Time.zone.parse("2026-09-24T02:00:00Z"))

    # 02:00 UTC is still September 23 in California.
    assert_equal "September 23, 2026", @user.ooo_until_date
  end

  test "OOO wins over a custom status, DND, and the meeting label" do
    @user.update!(ooo_until: 1.day.from_now,
      custom_status_emoji: "🚂", custom_status_text: "On a train",
      dnd_enabled: true,
      meeting_status_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])
    @user.reload

    assert @user.ooo_status_visible?
    assert_not @user.meeting_status_visible?
    assert_equal @user.ooo_status_text, @user.status_text_display
    assert_match "Out of office", @user.status_text_display
  end

  test "invisible hides the OOO label but OOO still reads as active" do
    @user.update!(ooo_until: 1.day.from_now, presence_setting: "invisible")

    assert @user.out_of_office?
    assert_not @user.ooo_status_visible?
    assert_nil @user.status_text_display
  end

  test "OOO quiet never suppresses the OOO label" do
    @user.update!(ooo_until: 1.day.from_now)

    assert @user.ooo_dnd_active?
    assert @user.ooo_status_visible?
  end

  test "calendar OOO needs the opt-in and a covering interval" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      ooo_intervals: [ [ 5.minutes.ago.iso8601, 2.days.from_now.iso8601 ] ])
    @user.reload

    assert_not @user.calendar_ooo_active?
    assert_not @user.out_of_office?

    @user.update!(ooo_calendar_enabled: true)

    assert @user.calendar_ooo_active?
    assert @user.out_of_office?
    assert_match "Out of office", @user.status_text_display
  end

  test "a calendar OOO outside its intervals reads as off" do
    @user.update!(ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      ooo_intervals: [ [ 2.days.ago.iso8601, 1.day.ago.iso8601 ] ])
    @user.reload

    assert_not @user.out_of_office?
  end

  test "overlapping manual and calendar OOO show the later end" do
    travel_to Time.zone.parse("2026-09-23 12:00") do
      @user.update!(ooo_calendar_enabled: true, ooo_until: Time.zone.parse("2026-09-24T12:00:00Z"))
      Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
        ooo_intervals: [ [ Time.zone.parse("2026-09-23T11:00:00Z").iso8601, Time.zone.parse("2026-09-26T12:00:00Z").iso8601 ] ])
      @user.reload

      assert_equal Time.zone.parse("2026-09-26T12:00:00Z"), @user.ooo_until_effective
      assert_equal "September 26, 2026", @user.ooo_until_date

      @user.update!(ooo_until: Time.zone.parse("2026-09-28T12:00:00Z"))

      assert_equal Time.zone.parse("2026-09-28T12:00:00Z"), @user.reload.ooo_until_effective
    end
  end

  test "the note shows only while the manual OOO is active" do
    @user.update!(ooo_calendar_enabled: true, ooo_until: 1.day.from_now, ooo_note: "Back soon")
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      ooo_intervals: [ [ 5.minutes.ago.iso8601, 3.days.from_now.iso8601 ] ])
    @user.reload

    assert_includes @user.ooo_status_text, "Back soon"

    travel_to 2.days.from_now do
      assert @user.reload.out_of_office?
      assert_not_includes @user.ooo_status_text, "Back soon"
    end
  end

  test "OOO presets run to the end of the day in the member's zone" do
    # 2026-09-23 is a Wednesday.
    travel_to Time.zone.parse("2026-09-23 12:00") do
      assert_equal Time.zone.parse("2026-09-24").end_of_day, @user.ooo_preset_until("tomorrow")
      assert_equal Time.zone.parse("2026-09-28").end_of_day, @user.ooo_preset_until("monday")
      assert_equal Time.zone.parse("2026-09-30").end_of_day, @user.ooo_preset_until("week")
    end
  end

  test "the Monday preset is a week out on Mondays" do
    travel_to Time.zone.parse("2026-09-28 12:00") do
      assert_equal Time.zone.parse("2026-10-05").end_of_day, @user.ooo_preset_until("monday")
    end
  end

  test "the custom preset parses a datetime-local value in the member's zone" do
    @user.update!(time_zone: "Pacific Time (US & Canada)")

    assert_equal Time.zone.parse("2026-10-01T15:30:00-07:00"),
      @user.ooo_preset_until("custom", "2026-10-01T15:30")
    assert_nil @user.ooo_preset_until("custom", "")
    assert_nil @user.ooo_preset_until("custom", "not-a-date")
  end

  test "an unknown preset raises" do
    assert_raises(ArgumentError) { @user.ooo_preset_until("someday") }
  end

  test "claim_ooo_broadcast! wins the first claim and each flip, and loses re-runs" do
    assert @user.claim_ooo_broadcast!(true)
    assert_not @user.claim_ooo_broadcast!(true)
    assert @user.claim_ooo_broadcast!(false)
    assert_not @user.claim_ooo_broadcast!(false)
  end

  test "claiming an end clears the expired manual columns" do
    @user.update!(ooo_until: 1.hour.from_now, ooo_note: "Back soon")
    assert @user.claim_ooo_broadcast!(true)

    travel_to 2.hours.from_now do
      assert @user.claim_ooo_broadcast!(false)
      assert_nil @user.reload.ooo_until
      assert_nil @user.reload.ooo_note
      assert_equal false, @user.reload.ooo_broadcast
    end
  end

  test "claiming an end keeps a manual OOO set racing the sweep" do
    @user.update_columns(ooo_until: 1.hour.ago, ooo_broadcast: true)
    @user.update!(ooo_until: 1.hour.from_now, ooo_note: "Back soon")

    assert_not @user.claim_ooo_broadcast!(false)
    assert_equal "Back soon", @user.reload.ooo_note
  end

  test "OOO quiets notifications unless the member keeps them on" do
    @user.update!(ooo_until: 1.day.from_now)

    assert @user.ooo_dnd_active?

    @user.update!(ooo_notify_enabled: true)

    assert_not @user.ooo_dnd_active?
  end

  test "deactivating clears the manual OOO columns" do
    @user.update!(ooo_until: 1.day.from_now, ooo_note: "Back soon")
    @user.claim_ooo_broadcast!(true)
    @user.reload

    @user.deactivate

    assert_nil @user.reload.ooo_until
    assert_nil @user.reload.ooo_note
    assert_nil @user.reload.ooo_broadcast
  end
end
