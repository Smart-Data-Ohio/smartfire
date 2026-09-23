require "test_helper"

class SecurityHeadersTest < ActionDispatch::IntegrationTest
  test "responses carry the required security headers" do
    get new_session_url

    assert_equal "nosniff", response.headers["X-Content-Type-Options"]
    assert_equal "strict-origin-when-cross-origin", response.headers["Referrer-Policy"]
    assert_equal "camera=(self), display-capture=(self), microphone=(self), notifications=(self)",
      response.headers["Permissions-Policy"]
  end

  test "authenticated pages carry the required security headers" do
    sign_in users(:kevin)
    get user_profile_url

    assert_equal "nosniff", response.headers["X-Content-Type-Options"]
    assert_equal "strict-origin-when-cross-origin", response.headers["Referrer-Policy"]
    assert_equal "camera=(self), display-capture=(self), microphone=(self), notifications=(self)",
      response.headers["Permissions-Policy"]
  end

  test "production enables HSTS through force_ssl" do
    # HSTS only has meaning over production TLS, so this pins the two
    # config lines that emit it (verified end to end against a
    # production boot: Strict-Transport-Security:
    # max-age=31556952; includeSubDomains).
    production = Rails.root.join("config/environments/production.rb").read

    assert_match(/config\.force_ssl\s*=\s*ENV\["DISABLE_SSL"\]\.blank\?/, production)
    assert_match(/config\.ssl_options\s*=\s*\{ hsts: \{ expires: 1\.year, subdomains: true \} \}/, production)
  end
end
