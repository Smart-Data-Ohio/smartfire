require "test_helper"

class Accounts::IntegrationsHealthControllerTest < ActionDispatch::IntegrationTest
  test "administrators see the health page" do
    sign_in :david
    GithubConnectedAccount.create!(user: users(:jz), github_login: "jz",
      access_token: "token", disconnected_reason: "revoked")

    get account_integrations_health_url

    assert_response :success
    assert_includes response.body, "Integration health"
    assert_includes response.body, "GitHub"
    assert_includes response.body, "Google Calendar"
    assert_includes response.body, "Fizzy"
    assert_includes response.body, "revoked"
  end

  test "plain members are forbidden" do
    sign_in :jz

    get account_integrations_health_url

    assert_response :forbidden
  end

  test "signed-out visitors are redirected to sign in" do
    get account_integrations_health_url

    assert_response :redirect
  end

  test "the page names what to set while unconfigured" do
    sign_in :david
    ENV.delete("GITHUB_APP_CLIENT_ID")
    ENV.delete("GITHUB_APP_CLIENT_SECRET")
    ENV.delete("GOOGLE_CALENDAR_WEBHOOK_URL")
    ENV.delete("INBOUND_EMAIL_DOMAIN")

    get account_integrations_health_url

    assert_response :success
    assert_includes response.body, "GITHUB_APP_CLIENT_ID"
    assert_includes response.body, "GOOGLE_CALENDAR_WEBHOOK_URL"
    assert_includes response.body, "INBOUND_EMAIL_DOMAIN"
  ensure
    ENV.delete("GITHUB_APP_CLIENT_ID")
    ENV.delete("GITHUB_APP_CLIENT_SECRET")
  end

  test "tokens and secrets are never rendered" do
    sign_in :david
    ENV["GITHUB_APP_CLIENT_ID"] = "app-id-for-health-test"
    ENV["GITHUB_APP_CLIENT_SECRET"] = "app-secret-for-health-test"
    GithubConnectedAccount.create!(user: users(:jz), github_login: "jz",
      access_token: "super-secret-token", token_source: "app",
      refresh_token: "super-secret-refresh", token_expires_at: 1.hour.from_now)

    get account_integrations_health_url

    assert_response :success
    assert_not_includes response.body, "super-secret-token"
    assert_not_includes response.body, "super-secret-refresh"
    assert_not_includes response.body, "app-secret-for-health-test"
  ensure
    ENV.delete("GITHUB_APP_CLIENT_ID")
    ENV.delete("GITHUB_APP_CLIENT_SECRET")
  end
end
