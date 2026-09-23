require "test_helper"

class TwoFactor::SetupsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:david)
  end

  test "show redirects visitors to sign in" do
    get two_factor_setup_url

    assert_redirected_to new_session_url
  end

  test "show renders the QR code and manual key for unenrolled users" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url

    assert_response :success
    assert_select "svg", count: 1
    assert_select "#two_factor_manual_key", text: @user.two_factor_credential.formatted_secret
    assert_select "form[action='#{two_factor_setup_path}'] input[name='code']", count: 1
  end

  test "show redirects enrolled users to the profile" do
    enroll_two_factor!(@user)
    sign_in @user
    get two_factor_setup_url

    assert_redirected_to user_profile_url
  end

  test "create enables two-factor, shows backup codes once, and audits" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url
    credential = @user.reload.two_factor_credential

    assert_no_difference -> { TwoFactorCredential.count } do
      post two_factor_setup_url, params: { code: totp_code_for(credential) }
    end

    assert_response :success
    assert credential.reload.enabled?
    assert_select "#two_factor_backup_codes li", count: 10
    assert_select "a[download='smartfire-backup-codes.txt']", count: 1

    audit = AuditLog.find_by!(action: "two_factor.enable")
    assert_equal @user.id, audit.actor_id
    assert_equal @user.id, audit.target_id

    # The session is verified now: the app opens instead of setup.
    get root_url
    assert_response :redirect
    assert_not_equal two_factor_setup_url, response.location
  end

  test "create rejects a wrong code without enabling or auditing" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url

    assert_no_difference -> { AuditLog.where(action: "two_factor.enable").count } do
      post two_factor_setup_url, params: { code: "000000" }
    end

    assert_response :unprocessable_entity
    assert_not @user.reload.two_factor_enabled?
  end

  test "create redirects enrolled users to the profile" do
    enroll_two_factor!(@user)
    sign_in @user

    post two_factor_setup_url, params: { code: "000000" }

    assert_redirected_to user_profile_url
  end

  test "create is rate limited per IP" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url

    with_rate_limit_store do
      10.times { post two_factor_setup_url, params: { code: "000000" } }
      assert_response :unprocessable_entity

      post two_factor_setup_url, params: { code: "000000" }
      assert_response :too_many_requests
    end
  end

  test "create is rate limited per user across IPs" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url

    with_rate_limit_store do
      10.times.each do |index|
        post two_factor_setup_url, params: { code: "000000" }, env: { "REMOTE_ADDR" => "10.1.0.#{index}" }
      end
      assert_response :unprocessable_entity

      post two_factor_setup_url, params: { code: "000000" }, env: { "REMOTE_ADDR" => "10.9.9.9" }
      assert_response :too_many_requests
    end
  end

  test "destroy disables two-factor, revokes devices, audits, and forces re-enrollment" do
    credential = enroll_two_factor!(@user)
    TwoFactorBackupCode.regenerate_set!(credential)
    TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
    sign_in @user

    delete two_factor_setup_url

    assert_redirected_to two_factor_setup_url
    assert_not @user.reload.two_factor_enabled?
    assert_equal 0, TwoFactorBackupCode.count
    assert_equal 0, @user.two_factor_remembered_devices.count
    assert AuditLog.exists?(action: "two_factor.disable", target_id: @user.id)

    get root_url
    assert_redirected_to two_factor_setup_url
  end

  test "destroy without enrollment redirects to setup and audits nothing" do
    sign_in @user

    assert_no_difference -> { AuditLog.count } do
      delete two_factor_setup_url
    end

    assert_redirected_to two_factor_setup_url
  end
end
