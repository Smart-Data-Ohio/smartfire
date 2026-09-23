require "test_helper"

class Accounts::Users::TwoFactorResetsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @admin = users(:david)
    @user = users(:kevin)
  end

  test "admin resets a user's 2FA, signs them out everywhere, and audits" do
    credential = enroll_two_factor!(@user)
    TwoFactorBackupCode.regenerate_set!(credential)
    TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
    @user.sessions.create!(user_agent: "Browser", ip_address: "1.2.3.4")
    sign_in @admin

    post account_user_two_factor_reset_url(@user)

    assert_redirected_to edit_account_url
    assert_equal "Two-step sign-in reset for #{@user.name}. They will set it up again at next sign-in.", flash[:notice]
    assert_not @user.reload.two_factor_enabled?
    assert_equal 0, TwoFactorBackupCode.count
    assert_equal 0, @user.two_factor_remembered_devices.count
    assert_equal 0, @user.sessions.count
    audit = AuditLog.find_by!(action: "two_factor.reset")
    assert_equal @admin.id, audit.actor_id
    assert_equal @user.id, audit.target_id
  end

  test "reset user re-enrolls at next sign-in" do
    enroll_two_factor!(@user)
    sign_in @admin
    post account_user_two_factor_reset_url(@user)

    delete session_url
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    assert_redirected_to root_url

    get root_url
    assert_redirected_to two_factor_setup_url
  end

  test "admin cannot reset their own 2FA this way" do
    credential = enroll_two_factor!(@admin)
    sign_in @admin

    assert_no_difference -> { AuditLog.count } do
      post account_user_two_factor_reset_url(@admin)
    end

    assert_redirected_to edit_account_url
    assert @admin.reload.two_factor_enabled?
    assert credential.reload.present?
  end

  test "reset without enrollment changes nothing" do
    sign_in @admin

    assert_no_difference -> { AuditLog.count } do
      post account_user_two_factor_reset_url(@user)
    end

    assert_redirected_to edit_account_url
  end

  test "non-admin cannot reset anyone's 2FA" do
    enroll_two_factor!(@user)
    sign_in users(:jz)

    assert_no_difference -> { AuditLog.count } do
      post account_user_two_factor_reset_url(@user)
    end

    assert_response :forbidden
    assert @user.reload.two_factor_enabled?
  end

  test "reset redirects visitors to sign in" do
    enroll_two_factor!(@user)

    post account_user_two_factor_reset_url(@user)

    assert_redirected_to new_session_url
    assert @user.reload.two_factor_enabled?
  end

  test "reset refuses bots" do
    sign_in @admin

    assert_raises ActiveRecord::RecordNotFound do
      post account_user_two_factor_reset_url(users(:bender))
    end
  end
end
