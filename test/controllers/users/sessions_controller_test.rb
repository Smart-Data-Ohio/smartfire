require "test_helper"

class Users::SessionsControllerTest < ActionDispatch::IntegrationTest
  CHROME_MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"

  setup do
    sign_in users(:kevin)
    @current = Session.find_by!(token: parsed_cookies.signed[:session_token])
    @current.update!(user_agent: CHROME_MAC, ip_address: "192.0.2.10")
    @other = users(:kevin).sessions.create!(user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
      ip_address: "192.0.2.20", last_active_at: 2.days.ago)
  end

  test "index lists only your own sessions" do
    david_session = sessions(:david_safari)

    get user_sessions_url

    assert_response :success
    assert_select "li", text: /Chrome on macOS/
    assert_select "li", text: /Firefox on Windows/
    assert_select "li", text: /192\.0\.2\.10/
    assert_select "li", text: /2 days ago/
    assert_select "li", text: /\(this device\)/, count: 1
    assert_select "form[action=?]", user_session_path(@other), count: 1
    assert_select "form[action=?]", user_session_path(david_session), count: 0
  end

  test "index hides expired administrator sessions" do
    sign_in users(:david)
    expired = users(:david).sessions.create!(user_agent: CHROME_MAC, ip_address: "192.0.2.40", last_active_at: 8.days.ago)
    fresh = users(:david).sessions.create!(user_agent: CHROME_MAC, ip_address: "192.0.2.41")

    get user_sessions_url

    assert_response :success
    assert_select "form[action=?]", user_session_path(expired), count: 0
    assert_select "form[action=?]", user_session_path(fresh), count: 1
  end

  test "destroy signs out another session immediately and audit-logs it" do
    other_browser = open_session
    other_browser.post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
    other_browser.get user_profile_url
    other_browser.assert_response :success
    other_session = users(:kevin).sessions.order(:id).last

    assert_difference -> { AuditLog.where(action: "session.revoke").count }, +1 do
      delete user_session_url(other_session)
    end

    assert_redirected_to user_sessions_url
    assert_not Session.exists?(other_session.id)

    other_browser.get user_profile_url
    other_browser.assert_redirected_to new_session_url
  end

  test "destroy keeps you signed in on the surviving session" do
    delete user_session_url(@other)

    get user_profile_url
    assert_response :success
  end

  test "destroy on the current session signs you out" do
    delete user_session_url(@current)

    assert_redirected_to root_url
    assert_not Session.exists?(@current.id)

    get user_profile_url
    assert_redirected_to new_session_url
  end

  test "destroy on someone else's session is not found" do
    assert_raises(ActiveRecord::RecordNotFound) do
      delete user_session_url(sessions(:david_safari))
    end
  end

  test "revoke_others with no other sessions writes no audit row" do
    @other.destroy!

    assert_no_difference -> { AuditLog.where(action: "session.revoke_others").count } do
      delete revoke_others_user_sessions_url
    end

    assert_redirected_to user_sessions_url
    assert_equal "No other sessions to sign out.", flash[:notice]
  end

  test "revoke_others signs out every other session and audit-logs it" do
    third = users(:kevin).sessions.create!(user_agent: CHROME_MAC, ip_address: "192.0.2.30")

    assert_difference -> { AuditLog.where(action: "session.revoke_others").count }, +1 do
      delete revoke_others_user_sessions_url
    end

    assert_redirected_to user_sessions_url
    assert_not Session.exists?(@other.id)
    assert_not Session.exists?(third.id)
    assert Session.exists?(@current.id)

    get user_profile_url
    assert_response :success
  end
end
