require "test_helper"

class User::MeetingStatusTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
  end

  test "meeting status and quiet-during-meetings default off" do
    assert_not @user.meeting_status_enabled?
    assert_not @user.meeting_dnd_enabled?
  end

  test "in_meeting? needs the opt-in and a covering interval" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    assert_not @user.in_meeting?

    @user.update!(meeting_status_enabled: true)

    assert @user.in_meeting?
  end

  test "in_meeting? is false without a cache row" do
    @user.update!(meeting_status_enabled: true)

    assert_not @user.in_meeting?
  end

  test "the meeting label shows while in a meeting" do
    opt_in_covering_now

    assert @user.meeting_status_visible?
    assert_equal "📅 In a meeting", @user.status_text_display
  end

  test "a custom status wins over the meeting label" do
    opt_in_covering_now
    @user.update!(custom_status_emoji: "🚂", custom_status_text: "On a train")

    assert_not @user.meeting_status_visible?
    assert_equal "🚂 On a train", @user.status_text_display
  end

  test "an expired custom status yields to the meeting label" do
    opt_in_covering_now
    @user.update!(custom_status_emoji: "🚂", custom_status_text: "On a train",
      custom_status_expires_at: 1.minute.ago)

    assert @user.meeting_status_visible?
    assert_equal "📅 In a meeting", @user.status_text_display
  end

  test "manual DND wins over the meeting label" do
    opt_in_covering_now
    @user.update!(dnd_enabled: true)

    assert_not @user.meeting_status_visible?
    assert_nil @user.status_text_display
  end

  test "the DND presence wins over the meeting label" do
    opt_in_covering_now
    @user.update!(presence_setting: "dnd")

    assert_not @user.meeting_status_visible?
    assert_nil @user.status_text_display
  end

  test "quiet hours win over the meeting label" do
    opt_in_covering_now
    now_minute = (Time.current.seconds_since_midnight / 60).to_i
    @user.update!(time_zone: "UTC", quiet_hours_enabled: true,
      quiet_hours_start_minute: (now_minute - 30) % (24 * 60),
      quiet_hours_end_minute: (now_minute + 30) % (24 * 60))

    assert @user.quiet_hours_active?
    assert_not @user.meeting_status_visible?
    assert_nil @user.status_text_display
  end

  test "invisible hides the meeting label" do
    opt_in_covering_now
    @user.update!(presence_setting: "invisible")

    assert_not @user.meeting_status_visible?
    assert_nil @user.status_text_display
  end

  test "quiet-during-meetings never suppresses the meeting label" do
    opt_in_covering_now
    @user.update!(meeting_dnd_enabled: true)

    assert @user.meeting_dnd_active?
    assert @user.meeting_status_visible?
    assert_equal "📅 In a meeting", @user.status_text_display
  end

  test "quiet-during-meetings only works while meeting status is on" do
    @user.update!(meeting_dnd_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    assert_not @user.meeting_dnd_active?

    @user.update!(meeting_status_enabled: true)

    assert @user.meeting_dnd_active?
  end

  test "quiet-during-meetings applies through a custom status" do
    opt_in_covering_now
    @user.update!(meeting_dnd_enabled: true,
      custom_status_emoji: "🚂", custom_status_text: "On a train")

    assert @user.meeting_dnd_active?
  end

  test "quiet-during-meetings is off outside busy intervals" do
    @user.update!(meeting_status_enabled: true, meeting_dnd_enabled: true)
    Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
      busy_intervals: [ [ 2.hours.ago.iso8601, 1.hour.ago.iso8601 ] ])

    assert_not @user.meeting_dnd_active?
  end

  private
    def opt_in_covering_now
      @user.update!(meeting_status_enabled: true)
      Calendar::MeetingCache.create!(user: @user, fetched_at: Time.current,
        busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])
      @user.reload
    end
end
