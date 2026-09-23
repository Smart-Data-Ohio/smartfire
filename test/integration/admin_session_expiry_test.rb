require "test_helper"

class AdminSessionExpiryTest < ActionDispatch::IntegrationTest
  test "an administrator session idle past the timeout is destroyed on next request" do
    sign_in users(:david)
    session = Session.find_by!(token: parsed_cookies.signed[:session_token])
    session.update!(last_active_at: 8.days.ago)

    get root_url

    assert_redirected_to new_session_url
    assert_not Session.exists?(session.id)
  end

  test "an administrator session idle within the timeout keeps working" do
    sign_in users(:david)
    session = Session.find_by!(token: parsed_cookies.signed[:session_token])
    session.update!(last_active_at: 6.days.ago)

    get user_profile_url

    assert_response :success
    assert Session.exists?(session.id)
  end

  test "a member session never expires" do
    sign_in users(:kevin)
    session = Session.find_by!(token: parsed_cookies.signed[:session_token])
    session.update!(last_active_at: 365.days.ago)

    get user_profile_url

    assert_response :success
    assert Session.exists?(session.id)
  end

  test "the timeout follows configuration" do
    previous = Rails.configuration.x.admin_session_idle_timeout
    Rails.configuration.x.admin_session_idle_timeout = 30.days

    sign_in users(:david)
    session = Session.find_by!(token: parsed_cookies.signed[:session_token])
    session.update!(last_active_at: 8.days.ago)

    get user_profile_url

    assert_response :success
    assert Session.exists?(session.id)
  ensure
    Rails.configuration.x.admin_session_idle_timeout = previous
  end
end
