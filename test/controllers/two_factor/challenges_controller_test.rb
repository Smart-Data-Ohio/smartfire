require "test_helper"

class TwoFactor::ChallengesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:david)
    @credential = enroll_two_factor!(@user)
  end

  test "show renders the challenge for a pending user" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get two_factor_challenge_url

    assert_response :success
    assert_select "form[action='#{two_factor_challenge_path}'] input[name='code']", count: 1
    assert_select "form[action='#{two_factor_challenge_path}'] input[name='remember_device'][type='checkbox']", count: 1
  end

  test "password sign-in leaves enrolled users pending with no session" do
    assert_no_difference -> { Session.count } do
      post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    end

    assert_redirected_to two_factor_challenge_url
    assert cookies[:session_token].blank?
  end

  test "calling the app while pending redirects to the challenge, never the app" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get root_url
    assert_redirected_to two_factor_challenge_url

    get root_url, headers: { "Accept" => "application/json" }
    assert_redirected_to two_factor_challenge_url
  end

  test "create with a valid TOTP signs in and audits the second factor" do
    get room_url(rooms(:pets))
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    post two_factor_challenge_url, params: { code: totp_code_for(@credential) }

    assert_redirected_to room_url(rooms(:pets))
    assert cookies[:session_token].present?
    assert Session.find_by(token: parsed_cookies.signed[:session_token]).two_factor_verified?

    audit = AuditLog.where(action: "session.sign_in.success").order(:id).last
    assert_equal @user.id, audit.actor_id
    assert_equal({ "method" => "password", "two_factor" => "totp" }, audit.details)

    follow_redirect!
    assert_response :success
  end

  test "create with a backup code signs in and spends it" do
    codes = TwoFactorBackupCode.regenerate_set!(@credential)
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    assert_difference -> { @credential.backup_codes.unused.count }, -1 do
      post two_factor_challenge_url, params: { code: codes.first }
    end

    assert_redirected_to root_url
    audit = AuditLog.where(action: "session.sign_in.success").order(:id).last
    assert_equal "backup_code", audit.details["two_factor"]

    # The spent code is dead: a fresh pending attempt with it fails.
    delete session_url
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    post two_factor_challenge_url, params: { code: codes.first }

    assert_response :unprocessable_entity
    assert cookies[:session_token].blank?
  end

  test "create rejects a wrong code, audits the failure, and never logs the code" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    assert_no_difference -> { Session.count } do
      post two_factor_challenge_url, params: { code: "s3cr3t-code" }
    end

    assert_response :unprocessable_entity
    assert cookies[:session_token].blank?

    audit = AuditLog.find_by!(action: "sign_in.two_factor.failure")
    assert_equal @user.id, audit.actor_id
    assert_equal "password", audit.details["method"]
    assert_not_includes audit.details.to_s, "s3cr3t-code"
  end

  test "create rejects reuse of the last accepted step" do
    travel_to Time.utc(2026, 9, 23, 12, 0, 10) do
      post session_url, params: { email_address: @user.email_address, password: "secret123456" }
      code = totp_code_for(@credential)
      post two_factor_challenge_url, params: { code: code }
      assert_redirected_to root_url

      delete session_url
      post session_url, params: { email_address: @user.email_address, password: "secret123456" }
      post two_factor_challenge_url, params: { code: code }

      assert_response :unprocessable_entity
      assert cookies[:session_token].blank?
    end
  end

  test "show and create without pending state redirect to sign in" do
    get two_factor_challenge_url
    assert_redirected_to new_session_url

    post two_factor_challenge_url, params: { code: totp_code_for(@credential) }
    assert_redirected_to new_session_url
  end

  test "expired pending state redirects to sign in" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    travel 11.minutes do
      get two_factor_challenge_url
      assert_redirected_to new_session_url

      post two_factor_challenge_url, params: { code: totp_code_for(@credential) }
      assert_redirected_to new_session_url
    end
  end

  test "pending user reset mid-challenge falls back to sign in" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    @user.reset_two_factor!

    get two_factor_challenge_url
    assert_redirected_to new_session_url
  end

  test "pending user deactivated mid-challenge cannot complete it" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    @user.update!(status: :deactivated)

    assert_no_difference -> { Session.count } do
      post two_factor_challenge_url, params: { code: totp_code_for(@credential) }
    end

    assert_redirected_to new_session_url
    assert cookies[:session_token].blank?
  end

  test "signed-in users are sent home from the challenge" do
    sign_in @user

    get two_factor_challenge_url
    assert_redirected_to root_url
  end

  test "remember device sets a signed, httponly, secure cookie bound to a record" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    post two_factor_challenge_url, params: { code: totp_code_for(@credential), remember_device: "1" }

    assert_redirected_to root_url
    assert_equal 1, @user.two_factor_remembered_devices.count

    set_cookies = Array(response.headers["Set-Cookie"]).join("\n").split("\n")
    remember = set_cookies.find { |line| line.start_with?("two_factor_remember=") }
    assert_predicate remember, :present?
    assert_match(/httponly/i, remember)
    assert_match(/secure/i, remember)
    assert_match(/samesite=lax/i, remember)
    assert_match(/expires=/i, remember)
  end

  # Rack::Test holds Secure cookies back over plain http, like a real
  # browser would outside localhost, so these flows run over https.
  test "remember cookie signs straight in without a challenge" do
    https!
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    post two_factor_challenge_url, params: { code: totp_code_for(@credential), remember_device: "1" }
    delete session_url

    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    assert_redirected_to root_url
    assert cookies[:session_token].present?
    audit = AuditLog.where(action: "session.sign_in.success").order(:id).last
    assert_equal "remembered_device", audit.details["two_factor"]
  end

  test "revoked remember cookie falls back to the challenge" do
    https!
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    post two_factor_challenge_url, params: { code: totp_code_for(@credential), remember_device: "1" }
    delete session_url
    @user.revoke_two_factor_remembered_devices!

    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    assert_redirected_to two_factor_challenge_url
    assert cookies[:session_token].blank?
  end

  test "create is rate limited per IP" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    with_rate_limit_store do
      10.times { post two_factor_challenge_url, params: { code: "000000" } }
      assert_response :unprocessable_entity

      post two_factor_challenge_url, params: { code: "000000" }
      assert_response :too_many_requests
    end
  end

  test "create is rate limited per user across IPs" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    with_rate_limit_store do
      10.times.each do |index|
        post two_factor_challenge_url, params: { code: "000000" }, env: { "REMOTE_ADDR" => "10.2.0.#{index}" }
      end
      assert_response :unprocessable_entity

      post two_factor_challenge_url, params: { code: "000000" }, env: { "REMOTE_ADDR" => "10.9.9.9" }
      assert_response :too_many_requests
    end
  end
end
