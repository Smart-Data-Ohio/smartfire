require "test_helper"

class ApplicationCable::ConnectionTest < ActionCable::Connection::TestCase
  test "connects with valid user_id cookie" do
    sessions(:david_safari).mark_two_factor_verified!
    cookies.signed[:session_token] = sessions(:david_safari).token

    connect

    assert_equal users(:david), connection.current_user
    assert_equal sessions(:david_safari), connection.current_session
  end

  test "rejects unenrolled humans whose session never completed two-step sign-in" do
    session = users(:david).sessions.create!(user_agent: "Test", ip_address: "1.2.3.4")
    cookies.signed[:session_token] = session.token

    assert_reject_connection { connect }
  end

  test "rejects stale unverified sessions of enrolled users" do
    enroll_two_factor!(users(:david))
    session = users(:david).sessions.create!(user_agent: "Test", ip_address: "1.2.3.4")
    cookies.signed[:session_token] = session.token

    assert_reject_connection { connect }
  end

  test "connects enrolled users whose session completed two-step sign-in" do
    enroll_two_factor!(users(:david))
    session = users(:david).sessions.create!(user_agent: "Test", ip_address: "1.2.3.4",
      two_factor_verified_at: Time.current)
    cookies.signed[:session_token] = session.token

    connect

    assert_equal users(:david), connection.current_user
  end

  test "rejects connection with missing user_id cookie" do
    assert_reject_connection { connect }
  end

  test "rejects connection with invalid user_id cookie" do
    cookies.signed[:session_token] = -1

    assert_reject_connection { connect }
  end

  test "rejects and destroys an expired administrator session" do
    sessions(:david_safari).update!(last_active_at: 8.days.ago)
    cookies.signed[:session_token] = sessions(:david_safari).token

    assert_reject_connection { connect }
    assert_not Session.exists?(sessions(:david_safari).id)
  end

  test "rejects connection for a revoked session" do
    token = sessions(:david_safari).token
    sessions(:david_safari).destroy!
    cookies.signed[:session_token] = token

    assert_reject_connection { connect }
  end

  test "connects an idle member session of any age" do
    session = users(:kevin).sessions.create!(user_agent: "test", ip_address: "127.0.0.1", last_active_at: 365.days.ago)
    cookies.signed[:session_token] = session.token

    connect

    assert_equal users(:kevin), connection.current_user
  end
end
