require "test_helper"

class TwoFactor::BackupCodesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:david)
  end

  test "regenerate replaces the set, shows it once, and audits" do
    credential = enroll_two_factor!(@user)
    old_codes = TwoFactorBackupCode.regenerate_set!(credential)
    sign_in @user

    post two_factor_backup_codes_url

    assert_response :success
    assert_select "#two_factor_backup_codes li", count: 10
    assert_select "a[href='#{user_profile_url}']", text: "Continue"
    assert_equal 10, credential.backup_codes.unused.count
    old_codes.each do |code|
      assert_not TwoFactorBackupCode.consume!(credential, code)
    end
    assert AuditLog.exists?(action: "two_factor.backup_codes.regenerate", target_id: @user.id)
  end

  test "regenerate without enrollment redirects to setup" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    post two_factor_backup_codes_url

    assert_redirected_to two_factor_setup_url
  end

  test "regenerate redirects visitors to sign in" do
    post two_factor_backup_codes_url

    assert_redirected_to new_session_url
  end
end
