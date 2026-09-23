require "test_helper"

class NewSignInAlertTest < ActionDispatch::IntegrationTest
  CHROME_MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"

  setup do
    @previous_smtp = ENV["SMTP_ADDRESS"]
    @previous_from = ENV["MAILER_FROM"]
    ENV["SMTP_ADDRESS"] = nil
    ENV["MAILER_FROM"] = nil
  end

  teardown do
    ENV["SMTP_ADDRESS"] = @previous_smtp
    ENV["MAILER_FROM"] = @previous_from
  end

  test "the first-ever sign-in does not alert" do
    assert_no_difference -> { ActivityItem.where(event_type: "new_sign_in").count } do
      post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
    end

    assert_response :redirect
    assert parsed_cookies.signed[:device_id].present?
  end

  test "signing in again from the same device does not alert" do
    post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }

    assert_no_difference -> { ActivityItem.where(event_type: "new_sign_in").count } do
      post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
    end
  end

  test "signing in from a new device creates an inbox item" do
    post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
    expire_device_cookie

    assert_difference -> { ActivityItem.where(event_type: "new_sign_in").count }, +1 do
      post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" },
        env: { "HTTP_USER_AGENT" => CHROME_MAC }
    end

    item = ActivityItem.where(event_type: "new_sign_in").last
    assert_equal users(:kevin), item.user
    assert_equal "Chrome on macOS", item.source.device_description

    get activity_items_url
    assert_response :success
    assert_select ".activity-item__body", text: /New sign-in to your account from Chrome on macOS/
  end

  test "the alert links to the sessions page" do
    post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
    expire_device_cookie
    post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }

    item = ActivityItem.where(event_type: "new_sign_in").last
    post open_activity_item_url(item)
    assert_redirected_to user_sessions_url
  end

  test "no email is enqueued when mail is not configured" do
    post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
    expire_device_cookie

    assert_no_enqueued_emails do
      post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
    end
  end

  test "an email is enqueued when mail is configured" do
    ENV["SMTP_ADDRESS"] = "smtp.example.com"
    ENV["MAILER_FROM"] = "Smartfire <alerts@example.com>"
    with_mail_host do
      post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
      expire_device_cookie

      assert_enqueued_jobs 1, only: ActionMailer::MailDeliveryJob do
        post session_url, params: { email_address: "kevin@37signals.com", password: "secret123456" }
      end
    end
  end

  private
    # A new browser has no device cookie: drop it so the next sign-in
    # mints a fresh id, like a device the account never used. The name
    # must be a string: Rack's jar ignores symbol deletes.
    def expire_device_cookie
      cookies.delete("device_id")
    end

    def with_mail_host
      Rails.application.routes.default_url_options[:host] = "example.com"
      yield
    ensure
      # Delete rather than nil-assign: an explicit nil host poisons every
      # _url helper in the process afterwards.
      Rails.application.routes.default_url_options.delete(:host)
    end
end
