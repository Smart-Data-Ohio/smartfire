require "test_helper"

class SecurityMailerTest < ActionMailer::TestCase
  include Rails.application.routes.url_helpers

  setup do
    Rails.application.routes.default_url_options[:host] = "example.com"
  end

  teardown do
    # Delete rather than nil-assign: an explicit nil host poisons every
    # _url helper in the process afterwards.
    Rails.application.routes.default_url_options.delete(:host)
  end

  test "configured? requires SMTP, a sender, and a URL host" do
    with_env("SMTP_ADDRESS" => "smtp.example.com", "MAILER_FROM" => "Smartfire <alerts@example.com>") do
      assert SecurityMailer.configured?
    end

    with_env("SMTP_ADDRESS" => nil, "MAILER_FROM" => "Smartfire <alerts@example.com>") do
      assert_not SecurityMailer.configured?
    end

    with_env("SMTP_ADDRESS" => "smtp.example.com", "MAILER_FROM" => nil) do
      assert_not SecurityMailer.configured?
    end
  end

  test "new_sign_in_alert names the device, time, and sessions page" do
    session = users(:kevin).sessions.create!(
      user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
      ip_address: "127.0.0.1", device_id: "device-1")
    item = ActivityItem.create!(user: users(:kevin), source: session, event_type: "new_sign_in")

    mail = nil
    with_env("SMTP_ADDRESS" => "smtp.example.com", "MAILER_FROM" => "Smartfire <alerts@example.com>") do
      mail = SecurityMailer.new_sign_in_alert(item)
    end

    assert_equal [ "kevin@37signals.com" ], mail.to
    assert_equal "New sign-in to your Smartfire account", mail.subject
    assert_match "New sign-in to your account from Chrome on macOS", mail.text_part.body.to_s
    assert_match "Wasn't you? Review your sessions", mail.text_part.body.to_s
    assert_match user_sessions_url, mail.text_part.body.to_s
    assert_match "New sign-in to your account from Chrome on macOS", mail.html_part.body.to_s
  end

  test "new_sign_in_alert skips cleanly when the session was revoked before delivery" do
    session = users(:kevin).sessions.create!(
      user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
      ip_address: "127.0.0.1", device_id: "device-1")
    item = ActivityItem.create!(user: users(:kevin), source: session, event_type: "new_sign_in")
    session.destroy!

    # The job deserializes a fresh item whose source row is gone.
    with_env("SMTP_ADDRESS" => "smtp.example.com", "MAILER_FROM" => "Smartfire <alerts@example.com>") do
      assert_nothing_raised do
        SecurityMailer.new_sign_in_alert(item.reload).deliver_now
      end
    end

    assert_empty ActionMailer::Base.deliveries
  end

  private
    def with_env(overrides)
      previous = overrides.keys.index_with { |key| ENV[key] }
      overrides.each { |key, value| ENV[key] = value }
      yield
    ensure
      previous.each { |key, value| ENV[key] = value }
    end
end
