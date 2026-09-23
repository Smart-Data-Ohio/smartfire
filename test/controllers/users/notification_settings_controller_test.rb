require "test_helper"

class Users::NotificationSettingsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "enables DND with quiet hours and keywords" do
    patch user_notification_settings_url, params: {
      user: {
        dnd_enabled: "1",
        quiet_hours_enabled: "1",
        quiet_hours_start: "22:00",
        quiet_hours_end: "07:00",
        keyword_alerts: "deploy\nlaunch day"
      }
    }

    assert_redirected_to user_profile_url
    user = users(:david).reload
    assert user.dnd_enabled?
    assert user.quiet_hours_enabled?
    assert_equal 22 * 60, user.quiet_hours_start_minute
    assert_equal 7 * 60, user.quiet_hours_end_minute
    assert_equal %w[ deploy launch\ day ], user.keyword_alerts.order(:phrase).pluck(:phrase)
  end

  test "disables DND and clears keywords" do
    users(:david).update!(dnd_enabled: true)
    KeywordAlert.create!(user: users(:david), phrase: "deploy")

    patch user_notification_settings_url, params: {
      user: { dnd_enabled: "0", quiet_hours_enabled: "0", keyword_alerts: "" }
    }

    assert_redirected_to user_profile_url
    user = users(:david).reload
    assert_not user.dnd_enabled?
    assert_empty user.keyword_alerts
  end

  test "enables quiet-during-meetings" do
    patch user_notification_settings_url, params: { user: { meeting_dnd_enabled: "1" } }

    assert_redirected_to user_profile_url
    assert users(:david).reload.meeting_dnd_enabled?
  end

  test "toggles keep-notifying while out of office, defaulting to off" do
    assert_not users(:david).ooo_notify_enabled?

    patch user_notification_settings_url, params: { user: { ooo_notify_enabled: "1" } }

    assert_redirected_to user_profile_url
    assert users(:david).reload.ooo_notify_enabled?

    patch user_notification_settings_url, params: { user: { ooo_notify_enabled: "0" } }

    assert_redirected_to user_profile_url
    assert_not users(:david).reload.ooo_notify_enabled?
  end

  test "quiet hours without a window render errors" do
    patch user_notification_settings_url, params: {
      user: { quiet_hours_enabled: "1", quiet_hours_start: "", quiet_hours_end: "" }
    }

    assert_response :unprocessable_entity
    assert_not users(:david).reload.quiet_hours_enabled?
  end

  test "a failed save keeps the previous keywords" do
    KeywordAlert.create!(user: users(:david), phrase: "deploy")

    patch user_notification_settings_url, params: {
      user: { quiet_hours_enabled: "1", quiet_hours_start: "", quiet_hours_end: "", keyword_alerts: "launch" }
    }

    assert_response :unprocessable_entity
    assert_equal [ "deploy" ], users(:david).reload.keyword_alerts.pluck(:phrase)
  end

  test "requires sign-in" do
    delete session_url

    patch user_notification_settings_url, params: { user: { dnd_enabled: "1" } }

    assert_redirected_to new_session_url
  end

  test "enabling DND after a timed expiry starts it indefinitely" do
    users(:david).update!(dnd_enabled: true, dnd_until: 1.hour.ago)

    patch user_notification_settings_url, params: { user: { dnd_enabled: "1" } }

    assert_redirected_to user_profile_url
    user = users(:david).reload
    assert_nil user.dnd_until
    assert user.manual_dnd_active?
  end

  test "disabling DND clears a running timer" do
    users(:david).update!(dnd_enabled: true, dnd_until: 1.hour.from_now)

    patch user_notification_settings_url, params: { user: { dnd_enabled: "0" } }

    assert_redirected_to user_profile_url
    user = users(:david).reload
    assert_nil user.dnd_until
    assert_not user.manual_dnd_active?
  end

  test "saving settings preserves a running DND timer" do
    users(:david).update!(dnd_enabled: true, dnd_until: 1.hour.from_now)

    patch user_notification_settings_url, params: { user: { dnd_enabled: "1" } }

    assert_redirected_to user_profile_url
    assert_in_delta 1.hour.from_now.to_f, users(:david).reload.dnd_until.to_f, 5
  end
end
