require "test_helper"

class TestSessionControllerTest < ActionDispatch::IntegrationTest
  test "signs in with valid credentials and lands on the post-auth page" do
    get test_sign_in_path(email_address: users(:david).email_address, password: "secret123456")
    assert_redirected_to root_url
    assert cookies[:session_token].present?

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

    get test_sign_in_path(email_address: users(:david).email_address, password: "secret123456")
    assert_response :not_found
    assert cookies[:session_token].blank?
  end
end
