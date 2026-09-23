require "application_system_test_case"

class TwoFactorTest < ApplicationSystemTestCase
  test "enrolling with an authenticator code opens the app" do
    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    assert_selector "h1", text: "Set up two-step sign-in", wait: 10
    assert_selector "#two_factor_manual_key"
    assert_selector "svg"

    fill_in "Authenticator code", with: totp_from_setup_page
    click_on "Verify and continue"

    assert_selector "h1", text: "Save your backup codes", wait: 10
    assert_selector "#two_factor_backup_codes li", count: 10
    click_on "Continue"

    assert_selector "aside#sidebar nav[aria-label='Your workspace']", wait: 10
    assert users(:jz).reload.two_factor_enabled?
  end

  test "signing in with an authenticator code" do
    credential = enroll_two_factor!(users(:jz))

    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    assert_selector "h1", text: "Enter your code", wait: 10
    fill_in "Authenticator or backup code", with: ROTP::TOTP.new(credential.secret).now
    click_on "Sign in"

    assert_selector "aside#sidebar nav[aria-label='Your workspace']", wait: 10
  end

  test "signing in with a backup code spends it" do
    credential = enroll_two_factor!(users(:jz))
    codes = TwoFactorBackupCode.regenerate_set!(credential)

    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    fill_in "Authenticator or backup code", with: codes.first
    click_on "Sign in"

    assert_selector "aside#sidebar nav[aria-label='Your workspace']", wait: 10
    assert TwoFactorBackupCode.find_by(code_digest: TwoFactorBackupCode.digest(codes.first)).used?

    visit user_profile_url
    click_on "Log out"

    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    fill_in "Authenticator or backup code", with: codes.first
    click_on "Sign in"

    assert_selector ".flash", text: "That code didn't work"
  end

  test "remembering the device skips the code until revoked" do
    credential = enroll_two_factor!(users(:jz))

    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    fill_in "Authenticator or backup code", with: ROTP::TOTP.new(credential.secret).now
    check "Remember this device for 30 days"
    click_on "Sign in"

    assert_selector "aside#sidebar nav[aria-label='Your workspace']", wait: 10
    visit user_profile_url
    click_on "Log out"

    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    assert_selector "aside#sidebar nav[aria-label='Your workspace']", wait: 10
    assert_no_selector "h1", text: "Enter your code"

    visit user_profile_url
    assert_selector "h2", text: "Two-step sign-in"
    within "menu li", match: :first do
      fill_in "Code or password", with: "secret123456"
      click_on "Revoke"
    end
    click_on "Log out"

    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    assert_selector "h1", text: "Enter your code", wait: 10
  end

  test "regenerating backup codes from the profile shows a fresh set" do
    credential = enroll_two_factor!(users(:jz))
    old_codes = TwoFactorBackupCode.regenerate_set!(credential)
    sign_in "jz@37signals.com"

    visit user_profile_url
    within "#new_backup_codes" do
      fill_in "Authenticator code or password", with: "secret123456"
      click_on "New backup codes"
    end

    assert_selector "h1", text: "Save your backup codes", wait: 10
    assert_selector "#two_factor_backup_codes li", count: 10
    shown = all("#two_factor_backup_codes li code").map(&:text)
    assert_empty shown & old_codes
  end

  test "disabling with the password drops back to setup and re-enrolling works" do
    enroll_two_factor!(users(:jz))
    sign_in "jz@37signals.com"

    visit user_profile_url
    within "#disable_two_factor" do
      fill_in "Authenticator code or password", with: "secret123456"
      click_on "Disable"
    end

    assert_selector "h1", text: "Set up two-step sign-in", wait: 10
    assert_not users(:jz).reload.two_factor_enabled?

    fill_in "Authenticator code", with: totp_from_setup_page
    click_on "Verify and continue"

    assert_selector "h1", text: "Save your backup codes", wait: 10
    assert users(:jz).reload.two_factor_enabled?
  end

  test "disabling without confirming leaves two-step sign-in on" do
    enroll_two_factor!(users(:jz))
    sign_in "jz@37signals.com"

    visit user_profile_url
    within "#disable_two_factor" do
      click_on "Disable"
    end

    assert_selector ".flash", text: "Enter your authenticator code", wait: 10
    assert users(:jz).reload.two_factor_enabled?
  end

  test "an admin resets a user's two-factor and the user re-enrolls" do
    enroll_two_factor!(users(:kevin))
    sign_in "david@37signals.com"

    visit edit_account_url
    accept_confirm { click_on "Reset two-step sign-in for Kevin" }

    assert_no_selector :button, "Reset two-step sign-in for Kevin", wait: 10
    assert_not users(:kevin).reload.two_factor_enabled?

    visit user_profile_url
    click_on "Log out"

    visit new_session_url
    fill_in "email_address", with: "kevin@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"

    assert_selector "h1", text: "Set up two-step sign-in", wait: 10
  end

  private
    def totp_from_setup_page
      secret = find("#two_factor_manual_key").text.gsub(/\s+/, "")
      ROTP::TOTP.new(secret).now
    end
end
