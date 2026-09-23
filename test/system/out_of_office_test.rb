require "application_system_test_case"

class OutOfOfficeTest < ApplicationSystemTestCase
  test "set OOO until tomorrow, badge and DM notice show for another user, then clear it" do
    sign_in "david@37signals.com"
    visit user_profile_url

    select "Until tomorrow", from: "user_ooo_preset"
    fill_in "user_ooo_note", with: "Back soon"
    within("form[action='#{user_status_path}']") { find("button[type='submit']", match: :first).click }
    wait_for_condition("OOO was not set") { users(:david).reload.manual_ooo_active? }

    visit user_url(users(:david))
    assert_selector ".user-status-badge", text: "Out of office"

    sign_in "jason@37signals.com"
    visit room_url(rooms(:david_and_jason))
    assert_selector ".ooo-notice", text: "David is out of office"
    assert_selector ".ooo-notice", text: "Back soon"

    sign_in "david@37signals.com"
    visit user_profile_url
    within("form[action='#{user_status_path}']") { click_on "Clear out of office" }
    wait_for_condition("OOO was not cleared") { !users(:david).reload.manual_ooo_active? }

    visit user_url(users(:david))
    assert_no_selector ".user-status-badge__custom", text: "Out of office"

    sign_in "jason@37signals.com"
    visit room_url(rooms(:david_and_jason))
    assert_no_selector ".ooo-notice"
  end

  private
    def wait_for_condition(message, timeout: 10)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
      until yield
        flunk message if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
        sleep 0.05
      end
      assert true
    end
end
