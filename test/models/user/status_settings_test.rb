require "test_helper"

class User::StatusSettingsTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
  end

  test "defaults to automatic presence, no DND, and the system theme" do
    assert_equal "auto", @user.presence_setting
    assert_not @user.dnd_enabled?
    assert_not @user.quiet_hours_enabled?
    assert_equal "system", @user.theme
    assert_nil @user.time_zone
  end

  test "rejects unknown presence, theme, and time zone values" do
    @user.presence_setting = "away"
    assert_not @user.valid?

    @user.presence_setting = "auto"
    @user.theme = "neon"
    assert_not @user.valid?

    @user.theme = "system"
    @user.time_zone = "Narnia"
    assert_not @user.valid?

    @user.time_zone = "Pacific Time (US & Canada)"
    assert @user.valid?
  end

  test "DND is manual-only outside quiet hours" do
    assert_not @user.dnd_active?

    @user.update!(dnd_enabled: true)
    assert @user.dnd_active?
  end

  test "the DND presence silences like the DND switch" do
    @user.update!(presence_setting: "dnd")

    assert @user.dnd_active?
    assert @user.notifications_muted?
    assert @user.notifications_muted?(sender: users(:jason))

    DndAllowedUser.create!(user: @user, allowed_user: users(:jason))
    assert_not @user.notifications_muted?(sender: users(:jason))
    assert @user.notifications_muted?(sender: users(:kevin))
  end

  test "quiet hours cover an overnight window in the user's time zone" do
    @user.update!(time_zone: "Pacific Time (US & Canada)",
      quiet_hours_enabled: true, quiet_hours_start: "22:00", quiet_hours_end: "07:00")

    travel_to Time.zone.parse("2026-09-23 06:30") do # 23:30 PDT
      assert @user.quiet_hours_active?
      assert @user.dnd_active?
    end

    travel_to Time.zone.parse("2026-09-23 16:00") do # 09:00 PDT
      assert_not @user.quiet_hours_active?
      assert_not @user.dnd_active?
    end
  end

  test "quiet hours need a start and an end while enabled" do
    @user.quiet_hours_enabled = true
    assert_not @user.valid?

    @user.quiet_hours_start = "22:00"
    @user.quiet_hours_end = "07:00"
    assert @user.valid?
    assert_equal 22 * 60, @user.quiet_hours_start_minute
    assert_equal 7 * 60, @user.quiet_hours_end_minute
  end

  test "malformed quiet-hours input reads as blank" do
    @user.quiet_hours_start = "late"
    assert_nil @user.quiet_hours_start_minute
    assert_nil @user.quiet_hours_start
  end

  test "quiet-hours input tolerates seconds" do
    @user.quiet_hours_start = "22:00:00"
    assert_equal 22 * 60, @user.quiet_hours_start_minute
    assert_equal "22:00", @user.quiet_hours_start
  end

  test "an expired custom status reads as blank" do
    @user.update!(custom_status_emoji: "🚂", custom_status_text: "On a train",
      custom_status_expires_at: 1.hour.from_now)

    assert @user.custom_status_active?
    assert_equal "🚂 On a train", @user.custom_status_display

    travel_to 2.hours.from_now do
      assert_not @user.custom_status_active?
      assert_nil @user.custom_status_display
    end
  end

  test "custom status expiry presets resolve in the user's time zone" do
    @user.update!(time_zone: "Pacific Time (US & Canada)")

    travel_to Time.zone.parse("2026-09-23 12:00") do # 05:00 PDT
      @user.custom_status_expires_in = "minutes_30"
      assert_equal Time.zone.parse("2026-09-23 12:30"), @user.custom_status_expires_at

      @user.custom_status_expires_in = "today"
      expiry = @user.custom_status_expires_at.in_time_zone("Pacific Time (US & Canada)")
      assert_equal Date.new(2026, 9, 23), expiry.to_date
      assert_equal 23, expiry.hour

      @user.custom_status_expires_in = "never"
      assert_nil @user.custom_status_expires_at
    end
  end

  test "an unknown custom status expiry raises" do
    assert_raises(ArgumentError) { @user.custom_status_expires_in = "fortnight" }
  end

  test "replacing keywords strips, dedupes, and caps the list" do
    KeywordAlert.create!(user: @user, phrase: "stale")

    assert @user.replace_keyword_alerts([ "Deploy\n  deploy  \n\n Launch " ])
    assert_equal %w[ Deploy Launch ], @user.keyword_alerts.order(:phrase).pluck(:phrase)
  end

  test "replacing keywords past the cap fails with errors" do
    lines = (KeywordAlert::MAX_PER_USER + 5).times.map { |index| "phrase #{index}" }

    assert_not @user.replace_keyword_alerts(lines)
    assert @user.errors[:base].any?
  end

  test "effective presence folds the manual setting over the lease state" do
    assert_equal :online, @user.effective_presence(:online)
    assert_equal :idle, @user.effective_presence(:idle)
    assert_equal :offline, @user.effective_presence(:offline)

    @user.update!(presence_setting: "dnd")
    assert_equal :dnd, @user.effective_presence(:online)
    assert_equal :dnd, @user.effective_presence(:idle)
    assert_equal :offline, @user.effective_presence(:offline)

    @user.update!(presence_setting: "invisible")
    assert_equal :offline, @user.effective_presence(:online)
  end
end
