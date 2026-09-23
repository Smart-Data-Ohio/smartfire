require "application_system_test_case"

class SessionsManagementTest < ApplicationSystemTestCase
  setup do
    sign_in "david@37signals.com"
  end

  test "reviewing and revoking sessions from the profile" do
    other = users(:david).sessions.create!(user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
      ip_address: "192.0.2.20")

    visit user_profile_path
    click_link "Manage sessions"

    assert_selector "h1", text: "Your sessions"
    assert_selector "li", text: /\(this device\)/
    assert_selector "li", text: /Firefox on Windows/

    accept_confirm do
      within "li", text: "Firefox on Windows" do
        click_button "Sign out"
      end
    end

    assert_selector ".flash", text: "Signed out that session", wait: 10
    assert_selector "li", text: /Firefox on Windows/, count: 0
    assert_not Session.exists?(other.id)

    # Still signed in on this device.
    visit user_profile_path
    assert_link "Manage sessions", wait: 10
  end
end
