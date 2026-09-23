module SessionTestHelper
  def parsed_cookies
    ActionDispatch::Cookies::CookieJar.build(request, cookies.to_hash)
  end

  def sign_in(user)
    user = users(user) unless user.is_a? User
    get sign_in_for_tests_path(email_address: user.email_address, password: "secret123456")
    assert cookies[:session_token].present?
  end

  # Confirms sudo mode with the fixture password, so tests exercising
  # sudo-gated actions start from a verified session.
  def grant_sudo_access(password = "secret123456")
    post sudo_url, params: { password: password }
    assert_response :redirect
  end
end
