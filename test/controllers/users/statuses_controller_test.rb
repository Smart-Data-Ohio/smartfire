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

  test "the status popup renders your current status inside the card frame" do
    users(:david).update!(presence_setting: "dnd", custom_status_emoji: "🚂", custom_status_text: "On a train")

    get edit_user_status_url, headers: { "Turbo-Frame" => "user_card" }

    assert_response :success
    assert_select "turbo-frame#user_card form[action='#{user_status_path}']" do
      assert_select "select#status_popup_presence_setting option[selected][value='dnd']"
      assert_select "input#status_popup_custom_status_emoji[value='🚂']"
      assert_select "input#status_popup_custom_status_text[value='On a train']"
      assert_select "select#status_popup_custom_status_expires_in"
      assert_select "button", text: "Save"
      assert_select "button[name='user[clear_custom_status]']", text: "Clear status"
      assert_select "a[href='#{user_card_path(users(:david))}']", text: "Cancel"
    end
    assert_select "input[name='user[meeting_status_enabled]']", count: 0
    assert_select "select[name='user[ooo_preset]']", count: 0
  end

  test "saving from the popup returns the card and broadcasts the new badge" do
    streams = capture_turbo_stream_broadcasts([ users(:david), :status ]) do
      patch user_status_url, headers: { "Turbo-Frame" => "user_card" }, params: {
        user: { presence_setting: "dnd", custom_status_emoji: "🚂", custom_status_text: "On a train", custom_status_expires_in: "never" }
      }
    end

    assert_redirected_to user_card_url(users(:david))
    assert_response :see_other
    assert_equal "dnd", users(:david).reload.presence_setting
    assert_equal "🚂 On a train", users(:david).custom_status_display
    assert_equal 1, streams.size
    assert_includes streams.first.to_html, "🚂 On a train"
  end

  test "clearing from the popup leaves meeting and out-of-office settings alone" do
    users(:david).update!(custom_status_emoji: "🚂", custom_status_text: "On a train",
      meeting_status_enabled: true, ooo_until: 2.days.from_now, ooo_note: "Back soon")

    patch user_status_url, headers: { "Turbo-Frame" => "user_card" },
      params: { user: { presence_setting: "auto", custom_status_emoji: "🚂", custom_status_text: "On a train",
        custom_status_expires_in: "never", clear_custom_status: "1" } }

    assert_redirected_to user_card_url(users(:david))
    user = users(:david).reload
    assert_nil user.custom_status_display
    assert user.meeting_status_enabled?
    assert_equal "Back soon", user.ooo_note
    assert user.ooo_until.future?
  end

  test "an invalid save from the popup re-renders the popup with its error" do
    patch user_status_url, headers: { "Turbo-Frame" => "user_card" },
      params: { user: { presence_setting: "away" } }

    assert_response :unprocessable_entity
    assert_select "turbo-frame#user_card form[action='#{user_status_path}']"
    assert_select "p", text: /Presence/
    assert_select "#user_ooo_preset", count: 0
    assert_equal "auto", users(:david).reload.presence_setting
  end

  test "saving an unchanged status broadcasts nothing" do
    assert_no_turbo_stream_broadcasts [ users(:david), :status ] do
      patch user_status_url, params: { user: { presence_setting: "auto" } }
    end
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

  test "opting out while in a meeting broadcasts the cleared badge" do
    users(:david).update!(meeting_status_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    streams = capture_turbo_stream_broadcasts([ users(:david), :status ]) do
      patch user_status_url, params: { user: { meeting_status_enabled: "0" } }
    end

    assert_redirected_to user_profile_url
    assert_equal 1, streams.size
    assert_not_includes streams.first.to_html, "In a meeting"
  end

  test "opting out without cached intervals broadcasts nothing" do
    users(:david).update!(meeting_status_enabled: true)

    assert_no_turbo_stream_broadcasts [ users(:david), :status ] do
      patch user_status_url, params: { user: { meeting_status_enabled: "0" } }
    end
  end

  test "saving other status settings leaves meeting refreshes alone" do
    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      patch user_status_url, params: { user: { presence_setting: "dnd" } }
    end
  end

  test "sets out of office with a preset and a note, and broadcasts it" do
    travel_to Time.zone.parse("2026-09-23 12:00") do
      streams = capture_turbo_stream_broadcasts([ users(:david), :status ]) do
        capture_turbo_stream_broadcasts([ users(:david), :ooo_notice ]) do
          patch user_status_url, params: {
            user: { ooo_preset: "tomorrow", ooo_note: "Back soon" }
          }
        end
      end

      assert_redirected_to user_profile_url
      user = users(:david).reload
      assert_in_delta Time.zone.parse("2026-09-24").end_of_day.to_f, user.ooo_until.to_f, 1
      assert_equal "Back soon", user.ooo_note
      assert_equal 1, streams.size
      assert_includes streams.first.to_html, "Out of office"
    end
  end

  test "sets out of office with a custom date and time in the member's zone" do
    users(:david).update!(time_zone: "Pacific Time (US & Canada)")

    # Relative so the suite never ages past it; mid-afternoon stays
    # clear of daylight-saving transitions when parsed back.
    custom = 7.days.from_now.in_time_zone("Pacific Time (US & Canada)").change(hour: 15, min: 30, sec: 0)
    patch user_status_url, params: {
      user: { ooo_preset: "custom", ooo_until_custom: custom.strftime("%Y-%m-%dT%H:%M") }
    }

    assert_redirected_to user_profile_url
    assert_in_delta custom.to_f, users(:david).reload.ooo_until.to_f, 1
  end

  test "rejects an unknown OOO preset without saving anything" do
    patch user_status_url, params: {
      user: { presence_setting: "dnd", ooo_preset: "someday" }
    }

    assert_response :unprocessable_entity
    assert_equal "auto", users(:david).reload.presence_setting
    assert_nil users(:david).ooo_until
  end

  test "rejects a blank or past custom OOO end without saving anything" do
    travel_to Time.zone.parse("2026-09-23 12:00") do
      patch user_status_url, params: {
        user: { presence_setting: "dnd", ooo_preset: "custom", ooo_until_custom: "" }
      }
      assert_response :unprocessable_entity

      patch user_status_url, params: {
        user: { ooo_preset: "custom", ooo_until_custom: "2026-09-22T15:30" }
      }
      assert_response :unprocessable_entity
    end

    assert_equal "auto", users(:david).reload.presence_setting
    assert_nil users(:david).ooo_until
  end

  test "rejects an OOO note over 140 characters" do
    patch user_status_url, params: {
      user: { ooo_preset: "tomorrow", ooo_note: "x" * 141 }
    }

    assert_response :unprocessable_entity
    assert_nil users(:david).reload.ooo_until
  end

  test "clears out of office early and broadcasts the cleared state" do
    users(:david).update!(ooo_until: 1.day.from_now, ooo_note: "Back soon")

    badge = nil
    badge = capture_turbo_stream_broadcasts([ users(:david), :status ]) do
      capture_turbo_stream_broadcasts([ users(:david), :ooo_notice ]) do
        patch user_status_url, params: { user: { clear_ooo: "1" } }
      end
    end

    assert_redirected_to user_profile_url
    assert_nil users(:david).reload.ooo_until
    assert_nil users(:david).ooo_note
    assert_equal 1, badge.size
    assert_not_includes badge.first.to_html, "Out of office"
  end

  test "clearing early ends only the manual OOO while calendar OOO covers" do
    users(:david).update!(ooo_calendar_enabled: true, ooo_until: 1.day.from_now, ooo_note: "Back soon")
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      ooo_intervals: [ [ 5.minutes.ago.iso8601, 3.days.from_now.iso8601 ] ])

    patch user_status_url, params: { user: { clear_ooo: "1" } }

    assert_redirected_to user_profile_url
    user = users(:david).reload
    assert_nil user.ooo_until
    assert_nil user.ooo_note
    assert user.out_of_office?
    assert_in_delta 3.days.from_now.to_f, user.ooo_until_effective.to_f, 5
  end

  test "edits the OOO note alone" do
    users(:david).update!(ooo_until: 1.day.from_now, ooo_note: "Back soon")

    patch user_status_url, params: { user: { ooo_note: "Slower than hoped" } }

    assert_redirected_to user_profile_url
    user = users(:david).reload
    assert_equal "Slower than hoped", user.ooo_note
    assert user.manual_ooo_active?
  end

  test "opting into calendar OOO enqueues a first refresh" do
    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ users(:david).id ]) do
      patch user_status_url, params: { user: { ooo_calendar_enabled: "1" } }
    end

    assert_redirected_to user_profile_url
    assert users(:david).reload.ooo_calendar_enabled?
  end

  test "opting out of calendar OOO clears its intervals and broadcasts" do
    users(:david).update!(ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      ooo_intervals: [ [ 5.minutes.ago.iso8601, 2.days.from_now.iso8601 ] ])

    badge = nil
    badge = capture_turbo_stream_broadcasts([ users(:david), :status ]) do
      capture_turbo_stream_broadcasts([ users(:david), :ooo_notice ]) do
        patch user_status_url, params: { user: { ooo_calendar_enabled: "0" } }
      end
    end

    assert_redirected_to user_profile_url
    assert_not users(:david).reload.ooo_calendar_enabled?
    assert_nil users(:david).meeting_cache
    assert_equal 1, badge.size
    assert_not_includes badge.first.to_html, "Out of office"
  end

  test "opting out of calendar OOO keeps the row while meeting status is on" do
    users(:david).update!(meeting_status_enabled: true, ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ],
      ooo_intervals: [ [ 5.minutes.ago.iso8601, 2.days.from_now.iso8601 ] ])

    patch user_status_url, params: { user: { ooo_calendar_enabled: "0" } }

    assert_redirected_to user_profile_url
    cache = users(:david).reload.meeting_cache
    assert_empty cache.ooo_intervals
    assert_equal 1, cache.busy_intervals.size
  end

  test "opting out of meeting status keeps the row while calendar OOO is on" do
    users(:david).update!(meeting_status_enabled: true, ooo_calendar_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ],
      ooo_intervals: [ [ 5.minutes.ago.iso8601, 2.days.from_now.iso8601 ] ])

    patch user_status_url, params: { user: { meeting_status_enabled: "0" } }

    assert_redirected_to user_profile_url
    cache = users(:david).reload.meeting_cache
    assert_empty cache.busy_intervals
    assert_equal 1, cache.ooo_intervals.size
  end

  test "saving other status settings leaves calendar OOO refreshes alone" do
    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      patch user_status_url, params: { user: { ooo_note: "Back soon" } }
    end
  end
end
