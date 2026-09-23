require "application_system_test_case"

class StatusNotificationsTest < ApplicationSystemTestCase
  setup do
    sign_in "david@37signals.com"
  end

  test "setting presence and a custom status" do
    session = users(:david).sessions.create!(user_agent: "System test", ip_address: "127.0.0.1")
    WorkspacePresenceLease.establish(user: users(:david), session:)

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
    assert_selector "meta[name='notification-dnd'][content='muted']", visible: :all

    visit user_profile_url
    assert_selector "#user_dnd_enabled:checked", visible: :all
  end

  test "chat sounds follow the live quiet-hours window without a reload" do
    message = rooms(:designers).messages.create!(
      creator: users(:jason), body: "/play tada", client_message_id: "sound-gate-#{SecureRandom.hex(4)}"
    )
    now_minute = (Time.current.seconds_since_midnight / 60).to_i
    users(:david).update!(
      time_zone: "UTC", quiet_hours_enabled: true,
      quiet_hours_start_minute: (now_minute - 60) % 1440,
      quiet_hours_end_minute: (now_minute + 60) % 1440
    )

    visit room_url(rooms(:designers))
    assert_selector "meta[name='quiet-hours']", visible: :all
    page.execute_script <<~JS
      window.playedSounds = [];
      window.Audio = class {
        constructor(url) { window.playedSounds.push(url); }
        play() { return Promise.resolve(); }
      };
    JS

    within_message(message) { click_button "🔊" }
    sleep 0.5
    assert_empty page.evaluate_script("window.playedSounds")

    # Time passes out of the window; the page never reloads.
    opposite = (now_minute + 720) % 1440
    page.execute_script <<~JS
      document.querySelector("meta[name='quiet-hours']")
        .setAttribute("content", "#{(opposite - 30) % 1440}-#{(opposite + 30) % 1440}");
    JS

    within_message(message) { click_button "🔊" }
    wait_for_condition("the sound did not play after quiet hours ended") do
      page.evaluate_script("window.playedSounds.length") == 1
    end
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

  test "button icons follow the manual theme, not the OS" do
    users(:david).update!(theme: "light")

    emulate_color_scheme "dark"
    visit user_profile_url
    assert_selector "html[data-theme='light']", visible: :all
    assert_equal "none", icon_filter(".workspace-navigation__open img")
    assert_equal "invert(1)", appearance_submit_icon_filter

    users(:david).update!(theme: "dark")
    emulate_color_scheme "light"
    visit user_profile_url
    assert_selector "html[data-theme='dark']", visible: :all
    assert_equal "invert(1)", icon_filter(".workspace-navigation__open img")
    assert_equal "none", appearance_submit_icon_filter
  ensure
    emulate_color_scheme nil
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
    def icon_filter(selector)
      find(selector, visible: :all).evaluate_script("getComputedStyle(this).filter")
    end

    def appearance_submit_icon_filter
      find("#user_theme_dark", visible: :all).ancestor("form").find(".btn--reversed img", visible: :all)
        .evaluate_script("getComputedStyle(this).filter")
    end

    def emulate_color_scheme(scheme)
      features = scheme ? [ { name: "prefers-color-scheme", value: scheme } ] : []
      page.driver.browser.execute_cdp "Emulation.setEmulatedMedia", features:
    end

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
