require "test_helper"

class Users::StatusesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "updates presence and the custom status with an expiry" do
    travel_to Time.zone.parse("2026-09-23 12:00") do
      patch user_status_url, params: {
        user: {
          presence_setting: "dnd",
          custom_status_emoji: "🚂",
          custom_status_text: "On a train",
          custom_status_expires_in: "hour_1"
        }
      }

      assert_redirected_to user_profile_url
      user = users(:david).reload
      assert_equal "dnd", user.presence_setting
      assert_equal "🚂 On a train", user.custom_status_display
      assert_equal Time.zone.parse("2026-09-23 13:00"), user.custom_status_expires_at
    end
  end

  test "clears the custom status" do
    users(:david).update!(custom_status_emoji: "🚂", custom_status_text: "On a train")

    patch user_status_url, params: { user: { clear_custom_status: "1" } }

    assert_redirected_to user_profile_url
    assert_nil users(:david).reload.custom_status_display
  end

  test "rejects an unknown presence with errors" do
    patch user_status_url, params: { user: { presence_setting: "away" } }

    assert_response :unprocessable_entity
    assert_equal "auto", users(:david).reload.presence_setting
  end

  test "requires sign-in" do
    delete session_url

    patch user_status_url, params: { user: { presence_setting: "dnd" } }

    assert_redirected_to new_session_url
  end
end
