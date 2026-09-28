require "test_helper"

class Accounts::SlackImportsControllerTest < ActionDispatch::IntegrationTest
  include SlackImportUiTestHelper

  setup do
    sign_in :david
    @david = users(:david)
  end

  test "setup is admin-only" do
    delete session_path
    sign_in :kevin

    get account_slack_import_path
    assert_response :forbidden

    patch account_slack_import_path, params: { client_id: "x", client_secret: "y" }
    assert_response :forbidden

    delete account_slack_import_path
    assert_response :forbidden
  end

  test "setup shows the manifest with the callback URL and user scopes" do
    get account_slack_import_path

    assert_response :success
    assert_select "h1", "Slack import"
    manifest = JSON.parse(extract_manifest)
    assert_equal "Smartfire Import", manifest.dig("display_information", "name")
    assert_equal false, manifest.dig("settings", "org_deploy_enabled")
    assert_equal [ "http://www.example.com/slack/oauth/callback" ], manifest.dig("oauth_config", "redirect_urls")
    assert_equal Slack::OAuth::USER_SCOPES, manifest.dig("oauth_config", "scopes", "user")
    assert_not_includes (manifest.dig("oauth_config", "scopes") || {}).keys, "bot"
  end

  test "the client secret is never rendered back" do
    create_slack_workspace!(client_secret: "super-secret-value")

    get account_slack_import_path

    assert_response :success
    assert_not_includes response.body, "super-secret-value"
  end

  test "saving credentials requires sudo" do
    patch account_slack_import_path, params: { client_id: "new-id", client_secret: "new-secret" }

    assert_redirected_to new_sudo_url
    assert_nil SlackWorkspace.current
  end

  test "saving credentials creates the workspace and records who configured it" do
    grant_sudo_access

    patch account_slack_import_path, params: { client_id: " new-id ", client_secret: "new-secret" }

    assert_redirected_to account_slack_import_path
    workspace = SlackWorkspace.current
    assert_equal "new-id", workspace.client_id
    assert_equal "new-secret", workspace.client_secret
    assert_equal @david, workspace.configured_by
    assert_equal "slack.workspace.configure", AuditLog.last.action
  end

  test "a blank secret keeps the stored one" do
    grant_sudo_access
    create_slack_workspace!(client_id: "old-id", client_secret: "kept-secret")

    patch account_slack_import_path, params: { client_id: "new-id", client_secret: "" }

    assert_redirected_to account_slack_import_path
    assert_equal "kept-secret", SlackWorkspace.current.client_secret
  end

  test "invalid credentials re-render with errors" do
    grant_sudo_access

    patch account_slack_import_path, params: { client_id: "", client_secret: "" }

    assert_response :unprocessable_content
    assert_nil SlackWorkspace.current
  end

  test "removing credentials requires sudo" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)

    delete account_slack_import_path

    assert_redirected_to new_sudo_url
    assert_predicate SlackWorkspace.current, :app_configured?
  end

  test "removing credentials clears them and every connection but keeps runs" do
    grant_sudo_access
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID, team_name: SLACK_TEAM_NAME)
    connect_slack!(@david, workspace:)
    run = create_slack_import!(workspace:, user: @david, status: "completed")

    delete account_slack_import_path

    assert_redirected_to account_slack_import_path
    assert_equal "Slack credentials removed.", flash[:notice]
    assert_not_predicate SlackWorkspace.current, :app_configured?
    assert_nil SlackWorkspace.current.team_id
    assert_empty SlackConnection.all
    assert SlackImport.exists?(run.id)
    assert_equal "slack.workspace.remove_credentials", AuditLog.last.action
  end

  test "removing credentials is blocked while a run is active" do
    grant_sudo_access
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david, workspace:)
    create_slack_import!(workspace:, user: @david, status: "running")

    delete account_slack_import_path

    assert_redirected_to account_slack_import_path
    assert_match(/running import/, flash[:alert])
    assert_predicate SlackWorkspace.current, :app_configured?
    assert_predicate @david.reload.slack_connection, :connected?
  end

  private
    def extract_manifest
      raw = response.body[%r{<pre id="slack-app-manifest".*?><code>(.+?)</code>}m, 1]
      CGI.unescapeHTML(raw.to_s)
    end
end
