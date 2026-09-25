require "test_helper"

class TestSessionControllerTest < ActionDispatch::IntegrationTest
  test "signs in with valid credentials and lands on the post-auth page" do
    get sign_in_for_tests_path(email_address: users(:david).email_address, password: "secret123456")
    assert_redirected_to root_url
    assert cookies[:session_token].present?

    # Root forwards a signed-in user to a room.
    follow_redirect!
    follow_redirect!
    assert_response :success
  end

  test "rejects invalid credentials" do
    get sign_in_for_tests_path(email_address: users(:david).email_address, password: "wrong-password")
    assert_response :unauthorized
    assert cookies[:session_token].blank?
  end

  test "bounces back to the HTML page visited before sign in" do
    get room_url(rooms(:hq))
    assert_redirected_to new_session_url

    get sign_in_for_tests_path(email_address: users(:david).email_address, password: "secret123456")
    assert_redirected_to room_url(rooms(:hq))
  end

  test "an unauthenticated JSON poll is not kept as the post-sign-in destination" do
    # A background poll firing after sign-out (or with an expired session)
    # still redirects to sign in, like every other unauthenticated request;
    # but its URL must never become the landing page, or sign-in drops the
    # member on a raw JSON document instead of the app.
    get unread_count_activity_items_url(format: :json)
    assert_redirected_to new_session_url

    get sign_in_for_tests_path(email_address: users(:david).email_address, password: "secret123456")
    assert_redirected_to root_url
  end

  test "an unauthenticated Turbo Stream poll or form post is not kept as the post-sign-in destination" do
    get activity_items_url(format: :turbo_stream)
    assert_redirected_to new_session_url

    post room_messages_url(rooms(:hq)), params: { message: { body: "hi" } }
    assert_redirected_to new_session_url

    get sign_in_for_tests_path(email_address: users(:david).email_address, password: "secret123456")
    assert_redirected_to root_url
  end

  test "is unreachable outside the test environment" do
    # Build the path before stubbing Rails.env. Routes load lazily on first
    # use, and config/routes.rb only draws the test sign-in route when
    # Rails.env.test?; if this were the first route lookup in a worker, the
    # stub would make routes load without it for every later test there.
    path = sign_in_for_tests_path(email_address: users(:david).email_address, password: "secret123456")
    Rails.stubs(:env).returns(ActiveSupport::StringInquirer.new("production"))

    with_public_exceptions do
      get path
      assert_response :not_found
    end
    assert cookies[:session_token].blank?
  end

  private
    # The test environment lets exceptions propagate; render them the way
    # production does so the guard's RoutingError surfaces as a 404.
    def with_public_exceptions
      env_config = Rails.application.env_config
      original = env_config["action_dispatch.show_exceptions"]
      env_config["action_dispatch.show_exceptions"] = :all
      yield
    ensure
      env_config["action_dispatch.show_exceptions"] = original
    end
end
