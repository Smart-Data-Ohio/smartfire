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

  test "opting into meeting status enqueues a first refresh" do
    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ users(:david).id ]) do
      patch user_status_url, params: { user: { meeting_status_enabled: "1" } }
    end

    assert_redirected_to user_profile_url
    assert users(:david).reload.meeting_status_enabled?
  end

  test "opting out of meeting status drops the cached intervals" do
    users(:david).update!(meeting_status_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      busy_intervals: [ [ 1.hour.ago.iso8601, 1.hour.from_now.iso8601 ] ])

    patch user_status_url, params: { user: { meeting_status_enabled: "0" } }

    assert_redirected_to user_profile_url
    assert_not users(:david).reload.meeting_status_enabled?
    assert_nil users(:david).meeting_cache
  end

  test "saving other status settings leaves meeting refreshes alone" do
    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      patch user_status_url, params: { user: { presence_setting: "dnd" } }
    end
  end
end
