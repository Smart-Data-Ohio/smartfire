require "test_helper"

class SessionTest < ActiveSupport::TestCase
  CHROME_MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
  FIREFOX_WINDOWS = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0"
  SAFARI_IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1"
  EDGE_WINDOWS = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0"

  test "expired? is false for a fresh administrator session" do
    assert_not sessions(:david_safari).expired?
  end

  test "expired? is true for an administrator session idle past the timeout" do
    sessions(:david_safari).update!(last_active_at: 8.days.ago)

    assert sessions(:david_safari).expired?
  end

  test "expired? is false for an administrator session idle within the timeout" do
    sessions(:david_safari).update!(last_active_at: 6.days.ago)

    assert_not sessions(:david_safari).expired?
  end

  test "expired? is always false for members" do
    session = users(:kevin).sessions.create!(user_agent: CHROME_MAC, ip_address: "127.0.0.1", last_active_at: 365.days.ago)

    assert_not session.expired?
  end

  test "expired? follows the configured timeout" do
    sessions(:david_safari).update!(last_active_at: 8.days.ago)

    with_admin_session_timeout(30.days) do
      assert_not sessions(:david_safari).expired?
    end
  end

  test "device_description names the browser and OS" do
    assert_equal "Chrome on macOS", build_session(CHROME_MAC).device_description
    assert_equal "Firefox on Windows", build_session(FIREFOX_WINDOWS).device_description
    assert_equal "Safari on iPhone", build_session(SAFARI_IPHONE).device_description
    assert_equal "Edge on Windows", build_session(EDGE_WINDOWS).device_description
  end

  test "device_description degrades gracefully without a user agent" do
    assert_equal "Unknown browser on Unknown device", build_session(nil).device_description
    assert_equal "Unknown browser on Unknown device", build_session("Rails Testing").device_description
  end

  private
    def build_session(user_agent)
      Session.new(user: users(:kevin), user_agent: user_agent)
    end

    def with_admin_session_timeout(timeout)
      previous = Rails.configuration.x.admin_session_idle_timeout
      Rails.configuration.x.admin_session_idle_timeout = timeout
      yield
    ensure
      Rails.configuration.x.admin_session_idle_timeout = previous
    end
end
