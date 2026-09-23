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
    secret = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last).secret
    assert_select "#two_factor_manual_key", text: secret.scan(/.{1,4}/).join(" ")
    assert_select "form[action='#{two_factor_setup_path}'] input[name='code']", count: 1
  end

  test "show issues a fresh secret on every visit" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url
    first = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last).secret

    get two_factor_setup_url
    second = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last).secret

    assert_not_equal first, second
  end

  test "show redirects enrolled users to the profile" do
    enroll_two_factor!(@user)
    sign_in @user
    get two_factor_setup_url

    assert_redirected_to user_profile_url
  end

  test "create enables two-factor, shows backup codes once, and audits" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get room_url(rooms(:pets))
    assert_redirected_to two_factor_setup_url
    get two_factor_setup_url
    setup_secret = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last)

    assert_difference -> { TwoFactorCredential.count }, 1 do
      post two_factor_setup_url, params: { code: totp_code_for_secret(setup_secret.secret) }
    end

    assert_response :success
    credential = @user.reload.two_factor_credential
    assert credential.enabled?
    assert_equal setup_secret.secret, credential.secret
    assert_nil TwoFactorSetupSecret.find_by(id: setup_secret.id)
    assert_select "#two_factor_backup_codes li", count: 10
    assert_select "a[download='smartfire-backup-codes.txt']", count: 1
    assert_select "a[href='#{room_url(rooms(:pets))}']", text: "Continue"

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

  test "create uses only this session's pending secret" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url
    first_secret = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last).secret

    other = open_session
    other.post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    other.get two_factor_setup_url
    second_secret = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last).secret
    assert_not_equal first_secret, second_secret

    # The first session's code does not confirm the second session.
    other.post two_factor_setup_url, params: { code: totp_code_for_secret(first_secret) }

    assert_equal 422, other.status
    assert_not @user.reload.two_factor_enabled?

    # The second session's own code confirms it.
    other.post two_factor_setup_url, params: { code: totp_code_for_secret(second_secret) }

    assert_equal 200, other.status
    assert @user.reload.two_factor_enabled?
    assert_equal second_secret, @user.two_factor_credential.secret
  end

  test "create rejects an expired pending secret and rotates it" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url
    setup_secret = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last)
    code = totp_code_for_secret(setup_secret.secret)
    setup_secret.update!(expires_at: 1.second.ago)

    post two_factor_setup_url, params: { code: code }

    assert_response :unprocessable_entity
    assert_not @user.reload.two_factor_enabled?
    rotated = TwoFactorSetupSecret.valid_for(@user.sessions.order(:id).last)
    assert_not_equal setup_secret.secret, rotated.secret
  end

  test "a wrong code keeps the live pending secret for retry" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    get two_factor_setup_url
    session = @user.sessions.order(:id).last
    before = TwoFactorSetupSecret.valid_for(session).secret

    post two_factor_setup_url, params: { code: "000000" }
    assert_response :unprocessable_entity

    after = TwoFactorSetupSecret.valid_for(session).secret
    assert_equal before, after

    post two_factor_setup_url, params: { code: totp_code_for_secret(after) }

    assert_response :success
    assert @user.reload.two_factor_enabled?
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
    other_session = @user.sessions.create!(user_agent: "Other", ip_address: "9.9.9.9", two_factor_verified_at: Time.current)
    sign_in @user

    delete two_factor_setup_url

    assert_redirected_to two_factor_setup_url
    assert_not @user.reload.two_factor_enabled?
    assert_equal 0, TwoFactorBackupCode.count
    assert_equal 0, @user.two_factor_remembered_devices.count
    assert_nil other_session.reload.two_factor_verified_at
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
