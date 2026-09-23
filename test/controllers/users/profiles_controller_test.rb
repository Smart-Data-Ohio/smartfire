require "test_helper"

class Users::ProfilesControllerTest < ActionDispatch::IntegrationTest
  include GoogleCalendarTestHelper

  setup do
    sign_in :david
  end

  test "show" do
    get user_profile_url

    assert_response :success
  end

  test "update" do
    put user_profile_url, params: { user: { name: "John Doe", bio: "Acrobat" } }

    assert_redirected_to user_profile_url
    assert_equal "John Doe", users(:david).reload.name
    assert_equal "Acrobat", users(:david).bio
    assert_equal "david@37signals.com", users(:david).email_address
  end

  test "updates are limited to the current user" do
    put user_profile_url(users(:jason)), params: { user: { name: "John Doe" } }

    assert_equal "Jason", users(:jason).reload.name
  end

  test "profile shows Google Calendar as not configured without credentials" do
    disconnect_google_env!

    get user_profile_url

    assert_includes response.body, "Google Calendar is not configured for this workspace"
    assert_not_includes response.body, "Connect Google Calendar"
  end

  test "profile offers a connect button without an account" do
    get user_profile_url

    assert_includes response.body, "Connect Google Calendar"
    assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1
  end

  test "profile links to connect for meeting status without an account" do
    get user_profile_url

    assert_response :success
    assert_select "a[href='#google-calendar-title']", text: "Connect Google Calendar"
    assert_select "input[name='user[meeting_status_enabled]'][type=checkbox]", count: 0
  end

  test "profile offers the meeting toggle for a connected account" do
    connect_google!(users(:david), email: "david@gmail.test")

    get user_profile_url

    assert_response :success
    assert_select "input[name='user[meeting_status_enabled]'][type=checkbox]", count: 1
    assert_includes response.body, "never titles or attendees"
  end

  test "profile shows the meeting fetch notice when a refresh failed" do
    connect_google!(users(:david), email: "david@gmail.test")
    users(:david).update!(meeting_status_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      fetch_error: Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)

    get user_profile_url

    assert_response :success
    assert_includes response.body, CGI.escapeHTML(Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)
  end

  test "profile asks to reconnect for meeting status left on after disconnect" do
    users(:david).update!(meeting_status_enabled: true)

    get user_profile_url

    assert_response :success
    assert_select "a[href='#google-calendar-title']", text: "Reconnect below"
  end

  test "profile lists the quiet-during-meetings switch" do
    get user_profile_url

    assert_response :success
    assert_select "input[name='user[meeting_dnd_enabled]'][type=checkbox]", count: 1
    assert_includes response.body, "Do not disturb during meetings"
  end

  test "the layout mutes sounds for quiet-during-meetings" do
    users(:david).update!(meeting_status_enabled: true, meeting_dnd_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    get user_profile_url

    assert_response :success
    assert_select "meta[name=notification-dnd][content=muted]", count: 1
  end

  test "the layout leaves sounds alone for meetings when quiet-during-meetings is off" do
    users(:david).update!(meeting_status_enabled: true)
    Calendar::MeetingCache.create!(user: users(:david), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    get user_profile_url

    assert_response :success
    assert_select "meta[name=notification-dnd]", count: 0
  end

  test "profile shows the connected account with a disconnect button" do
    connect_google!(users(:david), email: "david@gmail.test")

    get user_profile_url

    assert_includes response.body, "Connected as david@gmail.test"
    assert_includes response.body, "Disconnect"
  end

  test "profile offers a reconnect when Google rejected the connection" do
    connect_google!(users(:david), disconnected_reason: "Google rejected the connection")

    get user_profile_url

    assert_includes response.body, "Google rejected the connection, reconnect"
    assert_includes response.body, "Connect Google Calendar"
    assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1
  end

  test "profile offers Drive previews for a connected account without the Drive scope" do
    connect_google!(users(:david), email: "david@gmail.test")

    get user_profile_url

    assert_includes response.body, "Enable Drive previews"
    assert_not_includes response.body, "Drive previews enabled"
    assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1 do
      assert_select "input[name='features[]'][value=drive]", count: 1
    end
  end

  test "profile shows Drive previews as enabled when the account has the Drive scope" do
    connect_google!(users(:david), email: "david@gmail.test", scopes: DRIVE_SCOPES)

    get user_profile_url

    assert_includes response.body, "Drive previews enabled"
    assert_not_includes response.body, "Enable Drive previews"
  end

  test "profile offers Drive previews again for the retired metadata grant" do
    connect_google!(users(:david), email: "david@gmail.test", scopes: LEGACY_DRIVE_SCOPES)

    get user_profile_url

    assert_includes response.body, "Enable Drive previews"
    assert_not_includes response.body, "Drive previews enabled"
  end

  test "profile shows no Drive row when Google is not configured" do
    disconnect_google_env!

    get user_profile_url

    assert_not_includes response.body, "Enable Drive previews"
    assert_not_includes response.body, "Drive previews enabled"
  end

  test "profile asks to reconnect when the grant lacks the calendar scope" do
    connect_google!(users(:david), email: "david@gmail.test", scopes: "openid email")

    get user_profile_url

    assert_includes response.body, "Calendar permission needed, reconnect to publish events"
    assert_not_includes response.body, "Connected as"
    assert_includes response.body, "Disconnect"
    assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1
  end

  test "profile shows Disconnect for a partial grant with Drive still active" do
    connect_google!(users(:david), email: "david@gmail.test",
      scopes: "openid email #{Google::Client::DRIVE_SCOPE}")

    get user_profile_url

    assert_includes response.body, "Calendar permission needed, reconnect to publish events"
    assert_includes response.body, "Disconnect"
    assert_includes response.body, "Connect Google Calendar"
  end

  test "reconnect preserves a granted Drive scope" do
    connect_google!(users(:david), disconnected_reason: "Google rejected the connection", scopes: DRIVE_SCOPES)

    get user_profile_url

    assert_includes response.body, "Google rejected the connection, reconnect"
    assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1 do
      assert_select "input[name='features[]'][value=drive]", count: 1
    end
  end

  test "reconnect without Drive requests the calendar scope only" do
    connect_google!(users(:david), disconnected_reason: "Google rejected the connection")

    get user_profile_url

    assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1 do
      assert_select "input[name='features[]']", count: 0
    end
  end

  test "layout carries the Drive previews meta tag only with the Drive scope" do
    get user_profile_url
    assert_not_includes response.body, "google-drive-previews"

    connect_google!(users(:david), email: "david@gmail.test")
    get user_profile_url
    assert_not_includes response.body, "google-drive-previews"

    users(:david).google_account.update!(scopes: DRIVE_SCOPES)
    get user_profile_url
    assert_includes response.body, '<meta name="google-drive-previews" content="enabled">'
  end

  test "linking a github login strips and downcases it" do
    put user_profile_url, params: { user: { github_login: "  David-GH " } }

    assert_redirected_to user_profile_url
    assert_equal "david-gh", users(:david).reload.github_login
  end

  test "a github login cannot be claimed by a second user" do
    users(:jason).update!(github_login: "shared-login")

    put user_profile_url, params: { user: { github_login: "Shared-Login" } }

    assert_response :unprocessable_entity
    assert_select "p", text: /already linked to another user/
    assert_nil users(:david).reload.github_login
  end

  test "profile lists the notification switches with explanations" do
    get user_profile_url

    assert_response :success
    User::InboxPreferences::KEYS.each do |key|
      assert_select "input[name='user[inbox_preferences][#{key}]'][type=checkbox][checked]"
    end
    assert_includes response.body, "GitHub review requests"
    assert_includes response.body, "The incoming-call banner still shows."
  end

  test "profile saves the notification switches" do
    put user_profile_url, params: { user: { inbox_preferences: {
      "github_review_requests" => "0",
      "agent_approvals" => "0",
      "agent_work" => "1",
      "event_reminders" => "false",
      "huddle_invitations" => "true"
    } } }

    assert_redirected_to user_profile_url
    preferences = users(:david).reload.inbox_preferences
    assert_equal false, preferences.github_review_requests
    assert_equal false, preferences.agent_approvals
    assert_equal true, preferences.agent_work
    assert_equal false, preferences.event_reminders
    assert_equal true, preferences.huddle_invitations
  end

  test "profile rejects non-boolean notification input" do
    put user_profile_url, params: { user: { inbox_preferences: { "github_review_requests" => "banana" } } }

    assert_response :unprocessable_entity
    assert_equal true, users(:david).reload.inbox_preferences.github_review_requests
  end

  test "DND switch reflects the effective state after a timed expiry" do
    users(:david).update!(dnd_enabled: true, dnd_until: 1.hour.ago)

    get user_profile_url

    assert_response :success
    assert_select "#user_dnd_enabled[checked]", count: 0
  end

  test "DND switch stays on while a timer runs" do
    users(:david).update!(dnd_enabled: true, dnd_until: 1.hour.from_now)

    get user_profile_url

    assert_response :success
    assert_select "#user_dnd_enabled[checked]", count: 1
  end

  test "profile lists the call settings with their defaults" do
    get user_profile_url

    assert_response :success
    assert_select "select[name='user[voice_mode]'] option[selected]", text: "Voice activity"
    assert_select "input[name='user[push_to_talk_key]'][value='`']"
  end

  test "profile saves the call settings" do
    put user_profile_url, params: { user: { voice_mode: "push_to_talk", push_to_talk_key: "CapsLock" } }

    assert_redirected_to user_profile_url
    user = users(:david).reload
    assert_equal "push_to_talk", user.voice_mode
    assert_equal "CapsLock", user.push_to_talk_key
  end

  test "profile rejects an unknown microphone mode" do
    put user_profile_url, params: { user: { voice_mode: "shout" } }

    assert_response :unprocessable_entity
    assert_equal "voice_activity", users(:david).reload.voice_mode
  end

  test "clearing a github login unlinks it" do
    users(:david).update!(github_login: "david-gh")

    put user_profile_url, params: { user: { github_login: "" } }

    assert_redirected_to user_profile_url
    assert_nil users(:david).reload.github_login
  end

  test "changing email requires the current password" do
    put user_profile_url, params: { user: { email_address: "newhire@smartdata.net" } }

    assert_response :unprocessable_entity
    assert_includes response.body, "Current password is required to change your email address"
    assert_equal "david@37signals.com", users(:david).reload.email_address
    assert_nil users(:david).email_self_changed_at
  end

  test "changing email with a wrong current password is refused" do
    put user_profile_url, params: { user: { email_address: "newhire@smartdata.net", current_password: "wrong-password" } }

    assert_response :unprocessable_entity
    assert_includes response.body, "Current password is incorrect"
    assert_equal "david@37signals.com", users(:david).reload.email_address
  end

  test "a new password cannot stand in for the current one" do
    put user_profile_url, params: { user: { email_address: "newhire@smartdata.net", password: "brand-new-secret", current_password: "brand-new-secret" } }

    assert_response :unprocessable_entity
    assert_equal "david@37signals.com", users(:david).reload.email_address
    assert users(:david).authenticate("secret123456")
  end

  test "changing email with the current password records a self-change" do
    freeze_time do
      put user_profile_url, params: { user: { email_address: "david@smartdata.net", current_password: "secret123456" } }

      assert_redirected_to user_profile_url
      assert_equal "david@smartdata.net", users(:david).reload.email_address
      assert_equal Time.current, users(:david).email_self_changed_at
    end
  end

  test "other profile edits and case-only email edits need no password and record nothing" do
    put user_profile_url, params: { user: { name: "Dave", email_address: "David@37signals.com" } }

    assert_redirected_to user_profile_url
    assert_equal "Dave", users(:david).reload.name
    assert_nil users(:david).email_self_changed_at
  end

  test "profile asks for the current password only when the account has one" do
    get user_profile_url
    assert_select "input[name=?][autocomplete=current-password]", "user[current_password]"

    users(:david).update_columns(password_digest: nil)
    get user_profile_url
    assert_select "input[name=?]", "user[current_password]", count: 0
  end

  test "update saves the theme and time zone" do
    put user_profile_url, params: { user: { theme: "dark", time_zone: "Pacific Time (US & Canada)" } }

    assert_redirected_to user_profile_url
    assert_equal "dark", users(:david).reload.theme
    assert_equal "Pacific Time (US & Canada)", users(:david).time_zone
  end

  test "an IANA time zone round-trips through the form" do
    users(:david).update!(time_zone: "America/New_York")

    get user_profile_url
    assert_response :success
    assert_select "select#user_time_zone option[selected][value='America/New_York']"

    put user_profile_url, params: { user: { time_zone: "America/New_York" } }
    assert_redirected_to user_profile_url
    assert_equal "America/New_York", users(:david).reload.time_zone

    get user_profile_url
    assert_select "select#user_time_zone option[selected][value='America/New_York']"
  end

  test "a legacy Rails time zone name still shows selected" do
    users(:david).update!(time_zone: "Pacific Time (US & Canada)")

    get user_profile_url
    assert_response :success
    assert_select "select#user_time_zone option[selected][value='America/Los_Angeles']"
  end

  test "choosing a time zone or Not set records an explicit choice" do
    put user_profile_url, params: { user: { time_zone: "America/New_York" } }
    assert_redirected_to user_profile_url
    assert_equal "America/New_York", users(:david).reload.time_zone
    assert users(:david).time_zone_explicit?

    put user_profile_url, params: { user: { time_zone: "" } }
    assert_redirected_to user_profile_url
    assert_nil users(:david).reload.time_zone
    assert users(:david).time_zone_explicit?
  end

  test "the layout marks an explicit Not set so the browser skips detection" do
    users(:david).update!(time_zone_explicit: true)

    get user_profile_url
    assert_response :success
    assert_select "meta[name=current-user-time-zone][content='']", count: 1
  end

  test "update rejects an unknown theme or time zone" do
    put user_profile_url, params: { user: { theme: "neon" } }
    assert_response :unprocessable_entity

    put user_profile_url, params: { user: { time_zone: "Narnia" } }
    assert_response :unprocessable_entity

    assert_equal "system", users(:david).reload.theme
    assert_nil users(:david).time_zone
  end

  test "the layout carries the theme, time zone, and sound state" do
    users(:david).update!(theme: "light", time_zone: "Pacific Time (US & Canada)", dnd_enabled: true)

    get user_profile_url

    assert_response :success
    assert_select "html[data-theme=light]"
    assert_select "meta[name=color-scheme][content=light]", count: 1
    assert_select "meta[name=current-user-time-zone][content='Pacific Time (US & Canada)']", count: 1
    assert_select "meta[name=notification-dnd][content=muted]", count: 1
  end

  test "the layout mutes sounds for the DND presence" do
    users(:david).update!(presence_setting: "dnd")

    get user_profile_url

    assert_response :success
    assert_select "meta[name=notification-dnd][content=muted]", count: 1
  end

  test "the layout sends the quiet-hours window and zone for the sound gate" do
    users(:david).update!(time_zone: "UTC", quiet_hours_enabled: true,
      quiet_hours_start_minute: 1320, quiet_hours_end_minute: 420)

    get user_profile_url

    assert_response :success
    assert_select "meta[name=quiet-hours][content='1320-420']", count: 1
    assert_select "meta[name=quiet-hours-zone][content='UTC']", count: 1

    users(:david).update!(quiet_hours_enabled: false)

    get user_profile_url

    assert_response :success
    assert_select "meta[name=quiet-hours]", count: 0
    assert_select "meta[name=quiet-hours-zone]", count: 0
  end
end
