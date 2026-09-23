require "test_helper"

class TwoFactor::EnforcementTest < ActionDispatch::IntegrationTest
  include GoogleSignInTestHelper
  include GoogleCalendarTestHelper

  setup do
    @user = users(:david)
  end

  test "signed-in user without 2FA is redirected to setup before anything else" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get root_url
    assert_redirected_to two_factor_setup_url

    get room_url(rooms(:pets))
    assert_redirected_to two_factor_setup_url

    get user_profile_url
    assert_redirected_to two_factor_setup_url
  end

  test "unenrolled users can still open setup and sign out" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get two_factor_setup_url
    assert_response :success

    delete session_url
    assert_redirected_to root_url
    assert cookies[:session_token].blank?
  end

  test "static endpoints stay reachable while unenrolled" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get webmanifest_path(format: :json)
    assert_response :success

    get "/service-worker.js"
    assert_response :success
  end

  test "JSON requests cannot bypass the enrollment redirect" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get root_url, headers: { "Accept" => "application/json" }
    assert_response :forbidden
  end

  test "Turbo Stream requests cannot bypass the enrollment redirect" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get room_url(rooms(:pets)), headers: { "Accept" => "text/vnd.turbo-stream.html" }
    assert_redirected_to two_factor_setup_url
  end

  test "enrolled user with a stale unverified session is signed out to sign in" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    enroll_two_factor!(@user)

    assert_difference -> { Session.count }, -1 do
      get root_url
    end

    assert_redirected_to new_session_url
    assert cookies[:session_token].blank?
  end

  test "stale unverified sessions are rejected for JSON too" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    token = parsed_cookies.signed[:session_token]
    enroll_two_factor!(@user)

    get root_url, headers: { "Accept" => "application/json" }

    assert_response :unauthorized
    assert_nil Session.find_by(token: token)
  end

  test "verified sessions browse normally without 2FA (test sign-in)" do
    sign_in @user

    get root_url
    assert_response :redirect
    assert_not_equal two_factor_setup_url, response.location
  end

  test "verified enrolled sessions browse normally" do
    enroll_two_factor!(@user)
    sign_in @user

    get root_url
    assert_response :redirect
    assert_not_equal two_factor_setup_url, response.location
    assert_not_equal new_session_url, response.location
  end

  test "bot keys stay exempt from enforcement" do
    room = rooms(:pets)
    room.memberships.create!(user: users(:bender))

    post room_bot_messages_url(room, bot_key_for(users(:bender))), params: +"Hello Bot World!"

    assert_response :created
  end

  test "transfer links do not bypass 2FA for enrolled users" do
    credential = enroll_two_factor!(@user)

    assert_no_difference -> { Session.count } do
      put session_transfer_url(@user.transfer_id)
    end

    assert_redirected_to two_factor_challenge_url
    assert cookies[:session_token].blank?

    post two_factor_challenge_url, params: { code: totp_code_for(credential) }
    assert_redirected_to root_url
    audit = AuditLog.where(action: "session.sign_in.success").order(:id).last
    assert_equal "transfer", audit.details["method"]
    assert_equal "totp", audit.details["two_factor"]
  end

  test "transfer links still sign in unenrolled users" do
    put session_transfer_url(@user.transfer_id)

    assert_redirected_to root_url
    assert cookies[:session_token].present?
  end

  test "Google sign-in does not bypass 2FA for enrolled users" do
    user = User.create!(name: "Riel", email_address: "riel@smartdata.net", password: "secret123456",
      google_email_link_allowed: true)
    credential = enroll_two_factor!(user)

    state = start_google_sign_in
    assert_no_difference -> { Session.count } do
      complete_google_sign_in(state:, email: "riel@smartdata.net", hd: "smartdata.net", sub: "google-sub-riel")
    end

    assert_redirected_to two_factor_challenge_url
    assert cookies[:session_token].blank?

    post two_factor_challenge_url, params: { code: totp_code_for(credential) }
    assert_redirected_to root_url
    audit = AuditLog.where(action: "session.sign_in.success").order(:id).last
    assert_equal "google", audit.details["method"]
    assert_equal "totp", audit.details["two_factor"]
  end

  test "Google sign-in provisions unenrolled users into the setup flow" do
    state = start_google_sign_in
    complete_google_sign_in(state:, email: "alice@smartdata.net", hd: "smartdata.net", sub: "google-sub-alice")
    assert_redirected_to root_url

    get root_url
    assert_redirected_to two_factor_setup_url
  end

  test "stale sessions reaching the challenge are signed out at the next app request" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    enroll_two_factor!(@user)

    get two_factor_challenge_url
    assert_redirected_to root_url

    follow_redirect!
    assert_redirected_to new_session_url
  end

  test "Google callback terminates a stale session instead of sending it home" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }
    token = parsed_cookies.signed[:session_token]
    enroll_two_factor!(@user)

    get session_google_callback_url, params: { state: "bogus", code: "bogus" }

    assert_redirected_to new_session_url
    assert_nil Session.find_by(token: token)
  end

  test "join page sends unenrolled signed-in users to setup instead of home" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    get join_url(accounts(:signal).join_code)

    assert_redirected_to two_factor_setup_url
  end

  test "verified sessions still bounce off the join and challenge pages to home" do
    sign_in @user

    get join_url(accounts(:signal).join_code)
    assert_redirected_to root_url

    get two_factor_challenge_url
    assert_redirected_to root_url
  end
end
