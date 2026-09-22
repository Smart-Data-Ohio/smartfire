require "test_helper"

# "Link Google sign-in" from a signed-in member's own profile: the same
# verified flow as sign-in, bound to the member who started it.
class Users::GoogleSignInLinksControllerTest < ActionDispatch::IntegrationTest
  include GoogleSignInTestHelper
  include GoogleCalendarTestHelper

  setup do
    users(:kevin).update!(google_email_link_allowed: false)
    sign_in :kevin
  end

  test "the profile offers the link and the flow links the verified subject to the signed-in member" do
    get user_profile_url
    assert_select "form[action=?]", user_google_sign_in_link_path

    state = start_link
    complete_google_sign_in(state:, email: "kevin.w@smartdata.net", hd: "smartdata.net", sub: "google-sub-kevin")

    assert_redirected_to user_profile_url
    assert_equal "Google sign-in linked to kevin.w@smartdata.net.", flash[:notice]
    assert_equal users(:kevin).id, GoogleIdentity.find_by!(subject: "google-sub-kevin").user_id
    assert_equal "kevin@37signals.com", users(:kevin).reload.email_address, "linking never changes the email"

    delete session_url
    state = start_google_sign_in
    complete_google_sign_in(state:, email: "kevin.w@smartdata.net", hd: "smartdata.net", sub: "google-sub-kevin")
    assert_redirected_to root_url
  end

  test "the link flow keeps sign-in's domain allowlist" do
    state = start_link
    complete_google_sign_in(state:, email: "kevin@gmail.com", hd: "gmail.com", sub: "google-sub-kevin")

    assert_redirected_to user_profile_url
    assert_match "Only @smartdata.net and @cnbssoftware.com Google accounts can be linked", flash[:alert]
    assert_nil users(:kevin).reload.google_identity
  end

  test "the link flow keeps sign-in's nonce check" do
    state = start_link
    complete_google_sign_in(state:, email: "kevin.w@smartdata.net", hd: "smartdata.net", sub: "google-sub-kevin", nonce: "forged")

    assert_redirected_to user_profile_url
    assert_nil users(:kevin).reload.google_identity
  end

  test "a Google account that already signs in as someone else is refused" do
    GoogleIdentity.create!(user: users(:jz), subject: "google-sub-jz", email: "jz@smartdata.net", domain: "smartdata.net")

    state = start_link
    complete_google_sign_in(state:, email: "jz@smartdata.net", hd: "smartdata.net", sub: "google-sub-jz")

    assert_redirected_to user_profile_url
    assert_equal "That Google account already signs in as another member.", flash[:alert]
    assert_equal users(:jz).id, GoogleIdentity.find_by!(subject: "google-sub-jz").user_id
  end

  test "a member already linked to another subject is refused" do
    GoogleIdentity.create!(user: users(:kevin), subject: "google-sub-old", email: "kevin@smartdata.net", domain: "smartdata.net")

    post user_google_sign_in_link_url
    assert_redirected_to user_profile_url
    assert_equal "Google sign-in is already linked.", flash[:notice]
    assert_nil session[:google_sign_in_request]
  end

  test "a link flow finished by a different signed-in member links nobody" do
    state = start_link
    delete session_url
    sign_in :jz
    # The browser session changed, so the stashed link flow is gone.
    complete_google_sign_in(state:, email: "jz@smartdata.net", hd: "smartdata.net", sub: "google-sub-jz")

    assert_redirected_to root_url
    assert_equal 0, GoogleIdentity.count
  end

  test "a link flow whose member signed out does not sign anyone in" do
    state = start_link
    flow = session[:google_sign_in_request]
    assert_equal "link", flow["purpose"]
    assert_equal users(:kevin).id, flow["user_id"]

    delete session_url
    get session_google_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to new_session_url
    assert_equal 0, GoogleIdentity.count
  end

  test "starting a link requires a signed-in member and CSRF protection" do
    delete session_url
    post user_google_sign_in_link_url
    assert_redirected_to new_session_url

    sign_in :kevin
    with_forgery_protection do
      assert_raises(ActionController::InvalidAuthenticityToken) { post user_google_sign_in_link_url }
    end
  end

  private
    def start_link
      post user_google_sign_in_link_url
      assert_response :redirect
      query = Rack::Utils.parse_query(URI(response.location).query)
      assert_equal "http://www.example.com/session/google/callback", query["redirect_uri"]
      assert_equal "S256", query["code_challenge_method"]
      @last_sign_in_nonce = query["nonce"]
      query["state"]
    end

    def with_forgery_protection
      original = ActionController::Base.allow_forgery_protection
      ActionController::Base.allow_forgery_protection = true
      yield
    ensure
      ActionController::Base.allow_forgery_protection = original
    end
end
