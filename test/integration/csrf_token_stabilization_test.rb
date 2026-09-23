require "test_helper"

# A fresh sign-in must establish the session CSRF token before any page
# renders. Otherwise the first room HTML and a Turbo-preview-driven sidebar
# request each generate their own token; the sidebar's commit lands last and
# silently invalidates the page meta, so the next PATCH/POST 422s with
# InvalidAuthenticityToken. That is the flaky timezone 422 in system tests
# (the Chrome profile persists across tests, so a previous test's room
# snapshot previews with tokenless cookies) and a rare real-user 422 after
# sign-ins that skip the login form render (OAuth, first run, invites).
class CsrfTokenStabilizationTest < ActionDispatch::IntegrationTest
  setup do
    @forgery_protection = ActionController::Base.allow_forgery_protection
    ActionController::Base.allow_forgery_protection = true
  end

  teardown do
    ActionController::Base.allow_forgery_protection = @forgery_protection
  end

  test "test sign-in commits a CSRF token with the session" do
    get sign_in_for_tests_path(email_address: users(:jz).email_address, password: "secret123456")
    assert_redirected_to root_url
    assert_not_nil session[:_csrf_token], "sign-in left the session without a CSRF token"
  end

  test "a stale-cookie sidebar request cannot rotate the established token" do
    get sign_in_for_tests_path(email_address: users(:jz).email_address, password: "secret123456")
    assert_redirected_to root_url

    # The Turbo-preview race: the sidebar fetch is dispatched with the
    # pre-room cookie jar, before the room response commits its token.
    stale_session_cookie = cookies["_campfire_session"]
    auth_cookie = cookies["session_token"]

    get room_path(rooms(:designers))
    assert_response :success
    room_token = session[:_csrf_token]
    assert_not_nil room_token

    sidebar = open_session do |sess|
      sess.cookies["_campfire_session"] = stale_session_cookie
      sess.cookies["session_token"] = auth_cookie
      sess.get user_sidebar_path("me")
    end
    assert_response :success
    assert_includes sidebar.response.body, "authenticity_token",
      "sidebar rendered no form, so it never touched the CSRF token"
    assert_equal room_token, sidebar.session[:_csrf_token],
      "stale-cookie sidebar generated its own token instead of reusing the established one"
  end

  test "real login preserves the login-form token" do
    get new_session_path
    assert_response :success
    form_token = session[:_csrf_token]
    assert_not_nil form_token

    post session_path, params: {
      email_address: users(:jz).email_address, password: "secret123456",
      authenticity_token: response.body[/name="authenticity_token" value="([^"]+)"/, 1]
    }
    assert_redirected_to root_url
    assert_equal form_token, session[:_csrf_token]
  end
end
