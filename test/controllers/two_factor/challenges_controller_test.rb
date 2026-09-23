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

  test "five wrong codes lock the challenge, audit it, and notify the inbox" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    4.times { post two_factor_challenge_url, params: { code: "000000" } }
    assert_response :unprocessable_entity

    assert_difference -> { AuditLog.where(action: "sign_in.two_factor.lockout").count }, 1 do
      assert_difference -> { ActivityItem.where(user: @user, event_type: "two_factor_lockout").count }, 1 do
        post two_factor_challenge_url, params: { code: "000000" }
      end
    end

    assert_response :too_many_requests
    assert_includes response.body, "Too many wrong codes"

    lockout = AuditLog.find_by!(action: "sign_in.two_factor.lockout")
    assert_equal @user.id, lockout.actor_id
    assert_equal "password", lockout.details["method"]
  end

  test "a correct code is refused while locked and spends nothing" do
    codes = TwoFactorBackupCode.regenerate_set!(@credential)
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    5.times { post two_factor_challenge_url, params: { code: "000000" } }

    post two_factor_challenge_url, params: { code: totp_code_for(@credential) }
    assert_response :too_many_requests
    assert cookies[:session_token].blank?

    post two_factor_challenge_url, params: { code: codes.first }
    assert_response :too_many_requests
    assert cookies[:session_token].blank?
    assert_equal 10, @credential.backup_codes.unused.count
  end

  test "lockouts escalate across windows and a success resets them" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    now = Time.current
    travel_to(now) { 5.times { post two_factor_challenge_url, params: { code: "000000" } } }
    assert_response :too_many_requests
    assert_in_delta now + 1.minute, @credential.reload.locked_until, 5.seconds
    lockout_items = ActivityItem.where(user: @user, event_type: "two_factor_lockout")
    assert_equal 1, lockout_items.count

    # A repeat lockout resurfaces the same item instead of stacking.
    lockout_items.first.mark_read!
    travel_to(now + 2.minutes) { 5.times { post two_factor_challenge_url, params: { code: "000000" } } }
    assert_in_delta now + 7.minutes, @credential.reload.locked_until, 5.seconds
    assert_equal 1, lockout_items.count
    assert_predicate lockout_items.first.reload, :unread?

    # The per-user rate window (10 attempts per 15 minutes) still
    # holds the first batch at +8 minutes, so the success moves past
    # it; the pending state (10 minutes) is refreshed with a fresh
    # first factor first. Failure counters live on the credential, so
    # the re-stashed pending state changes nothing under test.
    travel_to(now + 16.minutes) do
      post session_url, params: { email_address: @user.email_address, password: "secret123456" }
      post two_factor_challenge_url, params: { code: totp_code_for(@credential.reload) }
    end
    assert_redirected_to root_url
    assert_equal 0, @credential.reload.lockout_count
    assert_equal 0, @credential.consecutive_failures
    assert_nil @credential.locked_until
  end

  test "lockout does not email unless mail is configured" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    assert_no_enqueued_jobs only: ActionMailer::MailDeliveryJob do
      5.times { post two_factor_challenge_url, params: { code: "000000" } }
    end

    assert_response :too_many_requests
  end

  test "lockout emails when mail is configured" do
    previous_settings = Rails.application.config.action_mailer.smtp_settings
    Rails.application.config.action_mailer.smtp_settings = { address: "smtp.example.com" }
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    assert_enqueued_with(job: ActionMailer::MailDeliveryJob) do
      5.times { post two_factor_challenge_url, params: { code: "000000" } }
    end

    assert_response :too_many_requests
  ensure
    Rails.application.config.action_mailer.smtp_settings = previous_settings
  end

  test "create is rate limited per IP" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    with_rate_limit_store do
      4.times { post two_factor_challenge_url, params: { code: "000000" } }
      assert_response :unprocessable_entity

      # The 5th failure locks the challenge; locked posts answer 429 with
      # the lockout message instead of the wrong-code one.
      6.times { post two_factor_challenge_url, params: { code: "000000" } }
      assert_response :too_many_requests
      assert_includes response.body, "Too many wrong codes"

      # The 11th attempt never reaches the controller: the IP limiter
      # answers first, with its own message.
      post two_factor_challenge_url, params: { code: "000000" }
      assert_response :too_many_requests
      assert_includes response.body, "Too many attempts"
    end
  end

  test "create is rate limited per user across IPs" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    with_rate_limit_store do
      4.times.each do |index|
        post two_factor_challenge_url, params: { code: "000000" }, env: { "REMOTE_ADDR" => "10.2.0.#{index}" }
      end
      assert_response :unprocessable_entity

      6.times.each do |index|
        post two_factor_challenge_url, params: { code: "000000" }, env: { "REMOTE_ADDR" => "10.3.0.#{index}" }
      end
      assert_response :too_many_requests
      assert_includes response.body, "Too many wrong codes"

      post two_factor_challenge_url, params: { code: "000000" }, env: { "REMOTE_ADDR" => "10.9.9.9" }
      assert_response :too_many_requests
      assert_includes response.body, "Too many attempts"
    end
  end
end
