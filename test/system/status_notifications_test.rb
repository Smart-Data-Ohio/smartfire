require "application_system_test_case"

class StatusNotificationsTest < ApplicationSystemTestCase
  setup do
    sign_in "david@37signals.com"
  end

  test "setting presence and a custom status" do
    visit user_profile_url

    select "Do not disturb", from: "user_presence_setting"
    fill_in "user_custom_status_emoji", with: "🚂"
    fill_in "user_custom_status_text", with: "On a train"
    select "1 hour", from: "user_custom_status_expires_in"
    within("form[action='#{user_status_path}']") { find("button[type='submit']", match: :first).click }

    wait_for_condition("the status was not saved") do
      users(:david).reload.custom_status_display == "🚂 On a train"
    end
    assert_equal "dnd", users(:david).reload.presence_setting

    visit user_url(users(:david))
    assert_selector ".user-status-badge", text: "Do not disturb"
    assert_selector ".user-status-badge", text: "🚂 On a train"
  end

  test "enabling DND mutes sounds and persists quiet hours" do
    visit user_profile_url

    find("#user_dnd_enabled", visible: :all).ancestor("label").click
    within("form[action='#{user_notification_settings_path}']") { find("button[type='submit']", match: :first).click }
    wait_for_condition("DND was not enabled") { users(:david).reload.dnd_enabled? }

    visit room_url(rooms(:designers))
    assert_selector "meta[name='notification-sounds'][content='muted']", visible: :all

    visit user_profile_url
    assert_selector "#user_dnd_enabled:checked", visible: :all
  end

  test "switching the theme applies without a reload flash" do
    visit user_profile_url
    assert_selector "html[data-theme='system']", visible: :all

    choose "user_theme_dark"
    form = find("#user_theme_dark").ancestor("form")
    within(form) { find("button[type='submit']").click }
    wait_for_condition("the theme was not saved") { users(:david).reload.theme == "dark" }

    assert_selector "html[data-theme='dark']", visible: :all
    assert_selector "meta[name='color-scheme'][content='dark']", visible: :all
  end

  test "the status form works at phone width" do
    page.current_window.resize_to(390, 844)

    visit user_profile_url
    assert_selector "#user_presence_setting"
    assert_selector "#user_custom_status_text"
    assert_selector "#user_theme_system"
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  private
    def wait_for_condition(message, timeout: Capybara.default_max_wait_time)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
      until yield
        if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
          raise Minitest::Assertion, message
        end

        sleep 0.05
      end
    end
end
