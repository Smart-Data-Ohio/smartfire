require "test_helper"

class Slack::OAuthControllerTest < ActionDispatch::IntegrationTest
  include SlackImportUiTestHelper

  setup do
    sign_in :david
    grant_sudo_access
    @david = users(:david)
  end

  test "start redirects to Slack with user scopes and no bot scope" do
    create_slack_workspace!

    get slack_oauth_start_path

    assert_response :redirect
    uri = URI(response.location)
    assert_equal "slack.com", uri.host
    assert_equal "/oauth/v2/authorize", uri.path
    query = Rack::Utils.parse_query(uri.query)
    assert_equal "test-client-id", query["client_id"]
    assert_equal SLACK_SCOPES, query["user_scope"]
    assert_not_includes query.keys, "scope"
    assert_equal slack_oauth_callback_url, query["redirect_uri"]
    assert_predicate query["state"], :present?
    assert_not_includes query.keys, "team"
  end

  test "start pins the team once it is known" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID, team_name: SLACK_TEAM_NAME)

    get slack_oauth_start_path

    query = Rack::Utils.parse_query(URI(response.location).query)
    assert_equal SLACK_TEAM_ID, query["team"]
  end

  test "start without configured credentials redirects back" do
    get slack_oauth_start_path

    assert_redirected_to account_slack_import_path
    assert_equal "Set up the Slack app credentials first.", flash[:alert]
  end

  test "start requires sudo" do
    create_slack_workspace!
    delete session_path
    sign_in :david

    get slack_oauth_start_path

    assert_redirected_to new_sudo_url
  end

  test "callback success upserts the connection and sets the team" do
    workspace = create_slack_workspace!
    state = start_state
    stub_slack_code_exchange
    stub_slack_team_info

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_equal "Slack connected.", flash[:notice]
    connection = @david.reload.slack_connection
    assert_equal SLACK_USER_ID, connection.slack_user_id
    assert_equal "xoxp-granted", connection.access_token
    assert_equal SLACK_SCOPES, connection.scopes
    assert_nil connection.disconnected_reason
    assert_equal SLACK_TEAM_ID, workspace.reload.team_id
    assert_equal SLACK_TEAM_NAME, workspace.team_name
    assert_equal SLACK_TEAM_DOMAIN, workspace.team_domain
    assert_requested :post, SLACK_ACCESS_URL, body: hash_including({
      "client_id" => "test-client-id", "client_secret" => "test-client-secret",
      "code" => "auth-code", "redirect_uri" => slack_oauth_callback_url
    })
  end

  test "callback success without a team.info answer still connects" do
    create_slack_workspace!
    state = start_state
    stub_slack_code_exchange
    stub_request(:get, SLACK_TEAM_INFO_URL).to_return(status: 500, body: "{}")

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_equal SLACK_TEAM_ID, SlackWorkspace.current.team_id
    assert_nil SlackWorkspace.current.team_domain
    assert_predicate @david.reload.slack_connection, :connected?
  end

  test "callback clears a previous disconnected reason" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david, disconnected_reason: "Slack rejected the token")
    state = start_state
    stub_slack_code_exchange

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_nil @david.reload.slack_connection.disconnected_reason
  end

  test "callback with a state mismatch is rejected" do
    create_slack_workspace!

    get slack_oauth_callback_path, params: { state: "bogus", code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_equal "Slack connection expired. Try again.", flash[:alert]
    assert_nil @david.reload.slack_connection
  end

  test "callback state cannot be replayed" do
    create_slack_workspace!
    state = start_state
    stub_slack_code_exchange
    stub_slack_team_info

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }
    assert_redirected_to account_slack_import_path

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }
    assert_redirected_to account_slack_import_path
    assert_equal "Slack connection expired. Try again.", flash[:alert]
  end

  test "callback state is bound to the user who started it" do
    create_slack_workspace!
    state = start_state(return_to: slack_imports_path)
    stub_slack_code_exchange
    stub_slack_team_info
    sign_in :kevin

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to slack_imports_path
    assert_equal "Slack connection expired. Try again.", flash[:alert]
    assert_nil users(:kevin).reload.slack_connection
    assert_nil @david.reload.slack_connection
  end

  test "callback with access_denied stores nothing" do
    create_slack_workspace!
    state = start_state

    get slack_oauth_callback_path, params: { state:, error: "access_denied" }

    assert_redirected_to account_slack_import_path
    assert_equal "Slack connection was not approved.", flash[:alert]
    assert_nil @david.reload.slack_connection
  end

  test "callback from a different team is rejected" do
    create_slack_workspace!(team_id: "TOTHER", team_name: "Other")
    state = start_state
    stub_slack_code_exchange

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_match(/different workspace/, flash[:alert])
    assert_nil @david.reload.slack_connection
  end

  test "callback missing a required scope is rejected with the missing ones" do
    create_slack_workspace!
    state = start_state
    stub_slack_code_exchange(slack_exchange_body(scopes: "channels:history,channels:read"))

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_match(/Missing: /, flash[:alert])
    assert_includes flash[:alert], "users:read.email"
    assert_nil @david.reload.slack_connection
  end

  test "callback refuses a Slack account linked to another member" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(users(:kevin), workspace:, slack_user_id: SLACK_USER_ID)
    state = start_state
    stub_slack_code_exchange

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_equal "That Slack account is already connected to another Smartfire user.", flash[:alert]
    assert_nil @david.reload.slack_connection
    assert_equal users(:kevin).id, SlackConnection.find_by(slack_user_id: SLACK_USER_ID).user_id
  end

  test "callback rescues a duplicate connection raced in after the check" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    state = start_state
    stub_slack_code_exchange
    SlackConnection.any_instance.stubs(:save!).raises(ActiveRecord::RecordNotUnique)

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_equal "That Slack account is already connected to another Smartfire user.", flash[:alert]
    assert_nil @david.reload.slack_connection
  end

  test "callback fills a missing team name from team.info" do
    workspace = create_slack_workspace!
    state = start_state
    stub_slack_code_exchange(slack_exchange_body(team_name: nil))
    stub_slack_team_info

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
    assert_equal SLACK_TEAM_NAME, workspace.reload.team_name
    assert_equal SLACK_TEAM_DOMAIN, workspace.team_domain
  end

  test "a failed exchange returns to the page the flow started from" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    state = start_state(return_to: slack_imports_path)
    stub_request(:post, SLACK_ACCESS_URL)
      .to_return(status: 200, body: { ok: false, error: "invalid_code" }.to_json)

    get slack_oauth_callback_path, params: { state:, code: "bad-code" }

    assert_redirected_to slack_imports_path
    assert_equal "Could not connect Slack. Try again.", flash[:alert]
    assert_nil @david.reload.slack_connection
  end

  test "callback with a failed exchange stores nothing" do
    create_slack_workspace!
    state = start_state
    stub_request(:post, SLACK_ACCESS_URL)
      .to_return(status: 200, body: { ok: false, error: "invalid_code" }.to_json)

    get slack_oauth_callback_path, params: { state:, code: "bad-code" }

    assert_redirected_to account_slack_import_path
    assert_equal "Could not connect Slack. Try again.", flash[:alert]
    assert_nil @david.reload.slack_connection
  end

  test "callback returns to the personal page when the flow started there" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    state = start_state(return_to: slack_imports_path)
    stub_slack_code_exchange

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to slack_imports_path
  end

  test "an off-allowlist return_to falls back to the default page" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    state = start_state(return_to: "https://evil.test/phish")
    stub_slack_code_exchange

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to account_slack_import_path
  end

  test "a member's first connection does not name the workspace team" do
    create_slack_workspace!
    delete session_path
    sign_in :kevin
    grant_sudo_access
    state = start_state
    stub_slack_code_exchange
    stub_slack_team_info

    get slack_oauth_callback_path, params: { state:, code: "auth-code" }

    assert_redirected_to slack_imports_path
    assert_nil SlackWorkspace.current.team_id
    assert_predicate users(:kevin).reload.slack_connection, :connected?
  end

  test "the code, token, and secret parameters are filtered from request logs" do
    filter = ActiveSupport::ParameterFilter.new(Rails.application.config.filter_parameters)

    assert_equal "[FILTERED]", filter.filter(code: "auth-code")[:code]
    assert_equal "[FILTERED]", filter.filter(access_token: "xoxp-granted")[:access_token]
    assert_equal "[FILTERED]", filter.filter(client_secret: "shh")[:client_secret]
  end

  private
    def start_state(return_to: nil)
      get slack_oauth_start_path(return_to:)
      assert_response :redirect
      Rack::Utils.parse_query(URI(response.location).query)["state"]
    end
end
