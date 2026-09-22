require "test_helper"

class TestSessionControllerTest < ActionDispatch::IntegrationTest
  test "signs in with valid credentials and lands on the post-auth page" do
    get test_sign_in_path(email_address: users(:david).email_address, password: "secret123456")
    assert_redirected_to root_url
    assert cookies[:session_token].present?

    # Root forwards a signed-in user to a room.
    follow_redirect!
    follow_redirect!
    assert_response :success
  end

  test "rejects invalid credentials" do
    get test_sign_in_path(email_address: users(:david).email_address, password: "wrong-password")
    assert_response :unauthorized
    assert cookies[:session_token].blank?
  end

  test "is unreachable outside the test environment" do
    Rails.stubs(:env).returns(ActiveSupport::StringInquirer.new("production"))

    with_public_exceptions do
      get test_sign_in_path(email_address: users(:david).email_address, password: "secret123456")
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
