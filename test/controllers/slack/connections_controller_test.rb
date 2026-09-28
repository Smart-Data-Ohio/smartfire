require "test_helper"

class Slack::ConnectionsControllerTest < ActionDispatch::IntegrationTest
  include SlackImportUiTestHelper

  setup do
    sign_in :david
    grant_sudo_access
    @david = users(:david)
  end

  test "disconnect revokes remotely and destroys the connection" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connection = connect_slack!(@david)
    revoke = stub_slack_revoke

    delete slack_connection_path(return_to: account_slack_import_path)

    assert_redirected_to account_slack_import_path
    assert_equal "Slack disconnected.", flash[:notice]
    assert_requested revoke
    assert_not SlackConnection.exists?(connection.id)
  end

  test "disconnect redirects a member to the personal page" do
    delete session_path
    sign_in :kevin
    grant_sudo_access
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(users(:kevin))
    stub_slack_revoke

    delete slack_connection_path

    assert_redirected_to slack_imports_path
    assert_nil users(:kevin).reload.slack_connection
  end

  test "disconnect is blocked while one of the member's runs is active" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connection = connect_slack!(@david)
    create_slack_import!(user: @david, status: "running")
    revoke = stub_slack_revoke

    delete slack_connection_path(return_to: account_slack_import_path)

    assert_redirected_to account_slack_import_path
    assert_match(/running Slack import/, flash[:alert])
    assert SlackConnection.exists?(connection.id)
    assert_not_requested revoke
  end

  test "another member's active run does not block disconnect" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connection = connect_slack!(@david)
    create_slack_import!(user: users(:kevin), status: "running")
    stub_slack_revoke

    delete slack_connection_path(return_to: account_slack_import_path)

    assert_redirected_to account_slack_import_path
    assert_not SlackConnection.exists?(connection.id)
  end

  test "disconnect without a connection still redirects" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    revoke = stub_slack_revoke

    delete slack_connection_path(return_to: account_slack_import_path)

    assert_redirected_to account_slack_import_path
    assert_not_requested revoke
  end

  test "disconnect requires sudo" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david)
    delete session_path
    sign_in :david

    delete slack_connection_path

    assert_redirected_to new_sudo_url
    assert_predicate @david.reload.slack_connection, :connected?
  end
end
