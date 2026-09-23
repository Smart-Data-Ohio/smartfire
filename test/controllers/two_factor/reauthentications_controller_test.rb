require "test_helper"

class TwoFactor::ReauthenticationsControllerTest < ActionDispatch::IntegrationTest
  include GoogleSignInTestHelper
  include GoogleCalendarTestHelper

  setup do
    @user = users(:david)
    @credential = enroll_two_factor!(@user)
    @identity = link_google_identity!(@user)
  end

  test "create redirects linked members to Google" do
    sign_in @user

    post two_factor_reauthentication_path

    assert_response :redirect
    assert_equal "accounts.google.com", URI(response.location).host
  end

  test "create refuses members without a linked Google account" do
    @identity.destroy!
    sign_in @user

    post two_factor_reauthentication_path

    assert_redirected_to user_profile_url
    assert_equal "Google confirmation needs a linked Google account.", flash[:alert]
  end

  test "create refuses when Google is not configured" do
    Google::SignIn.stubs(:configured?).returns(false)
    sign_in @user

    post two_factor_reauthentication_path

    assert_redirected_to user_profile_url
    assert_equal "Google confirmation needs a linked Google account.", flash[:alert]
  end

  test "create forces a fresh Google sign-in" do
    sign_in @user

    post two_factor_reauthentication_path

    query = Rack::Utils.parse_query(URI(response.location).query)
    assert_equal "login", query["prompt"]
    assert_equal "0", query["max_age"]
  end

  test "callback arms a step-up and audits it" do
    sign_in @user

    state = start_google_reauth
    complete_google_sign_in(state:, sub: @identity.subject, email: @identity.email, auth_time: Time.current.to_i)

    assert_redirected_to user_profile_url
    assert_equal "Confirmed with Google. Continue with what you were doing.", flash[:notice]
    audit = AuditLog.find_by!(action: "two_factor.reauthenticate")
    assert_equal @user.id, audit.actor_id
    assert_equal @user.id, audit.target_id
  end

  test "callback refuses a different Google account" do
    sign_in @user

    state = start_google_reauth
    complete_google_sign_in(state:, sub: "google-sub-attacker", email: "attacker@smartdata.net", auth_time: Time.current.to_i)

    assert_redirected_to user_profile_url
    assert_equal "That Google account is not linked here. Confirm with the Google account you sign in with.", flash[:alert]
    assert_not AuditLog.exists?(action: "two_factor.reauthenticate")

    # Nothing was armed: the sensitive action is still refused.
    post two_factor_backup_codes_url
    assert_redirected_to user_profile_url
  end

  test "callback completed by another member arms nothing" do
    enroll_two_factor!(users(:jason))
    sign_in @user
    state = start_google_reauth

    delete session_url
    sign_in users(:jason)
    complete_google_sign_in(state:, sub: @identity.subject, email: @identity.email)

    assert_redirected_to root_url
    assert_not AuditLog.exists?(action: "two_factor.reauthenticate")
  end

  test "a completed re-auth expires after ten minutes" do
    sign_in @user

    state = start_google_reauth
    complete_google_sign_in(state:, sub: @identity.subject, email: @identity.email, auth_time: Time.current.to_i)
    assert_redirected_to user_profile_url

    travel_to 11.minutes.from_now do
      post two_factor_backup_codes_url
      assert_redirected_to user_profile_url
    end
  end

  test "callback refuses a stale Google authentication" do
    sign_in @user

    state = start_google_reauth
    complete_google_sign_in(state:, sub: @identity.subject, email: @identity.email, auth_time: 6.minutes.ago.to_i)

    assert_redirected_to user_profile_url
    assert_equal "Google confirmation failed. Try again.", flash[:alert]
    assert_not AuditLog.exists?(action: "two_factor.reauthenticate")

    # Nothing was armed: the sensitive action is still refused.
    post two_factor_backup_codes_url
    assert_redirected_to user_profile_url
  end

  test "callback refuses a token without auth_time" do
    sign_in @user

    state = start_google_reauth
    complete_google_sign_in(state:, sub: @identity.subject, email: @identity.email, auth_time: nil)

    assert_redirected_to user_profile_url
    assert_equal "Google confirmation failed. Try again.", flash[:alert]
    assert_not AuditLog.exists?(action: "two_factor.reauthenticate")
  end

  test "create signs out a stale unverified session instead of starting re-auth" do
    @credential.destroy!
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    token = parsed_cookies.signed[:session_token]
    enroll_two_factor!(@user)

    post two_factor_reauthentication_path

    assert_redirected_to new_session_url
    assert_nil Session.find_by(token: token)
  end

  test "create is rate limited per IP" do
    sign_in @user

    with_rate_limit_store do
      10.times { post two_factor_reauthentication_path }
      assert_equal "accounts.google.com", URI(response.location).host

      post two_factor_reauthentication_path
      assert_redirected_to user_profile_url
      assert_equal "Too many attempts. Try again in a few minutes.", flash[:alert]
    end
  end

  test "create is rate limited per user across IPs" do
    sign_in @user

    with_rate_limit_store do
      10.times.each do |index|
        post two_factor_reauthentication_path, env: { "REMOTE_ADDR" => "10.1.0.#{index}" }
      end
      assert_equal "accounts.google.com", URI(response.location).host

      post two_factor_reauthentication_path, env: { "REMOTE_ADDR" => "10.9.9.9" }
      assert_redirected_to user_profile_url
      assert_equal "Too many attempts. Try again in a few minutes.", flash[:alert]
    end
  end

  test "create redirects visitors to sign in" do
    post two_factor_reauthentication_path

    assert_redirected_to new_session_url
  end
end
