require "application_system_test_case"

class SlackImportTest < ApplicationSystemTestCase
  include SlackImportUiTestHelper

  setup do
    sign_in "david@37signals.com"
  end

  test "admin saves credentials, dry-runs, plans, and watches a test import" do
    visit account_slack_import_path
    assert_selector "h1", text: "Slack import"
    assert_selector "#slack-app-manifest", text: "Smartfire Import"

    # Saving credentials is sudo-gated; secret forms are not replayed,
    # so the admin submits once more after confirming.
    fill_in "Client ID", with: "test-client-id"
    fill_in "Client Secret", with: "test-client-secret"
    click_button "Save credentials"
    assert_selector "h1", text: "Confirm it's you", wait: 10
    fill_in "password", with: "secret123456"
    click_button "Confirm"
    assert_selector "h1", text: "Slack import", wait: 10
    fill_in "Client ID", with: "test-client-id"
    fill_in "Client Secret", with: "test-client-secret"
    click_button "Save credentials"
    assert_selector ".flash", text: "credentials saved", wait: 10

    # As if the admin connected Slack through OAuth: the workspace is
    # named and the admin holds a live connection.
    workspace = SlackWorkspace.current
    workspace.update!(team_id: SLACK_TEAM_ID, team_name: SLACK_TEAM_NAME)
    connect_slack!(users(:david), workspace:)

    visit account_slack_import_path
    click_button "Start dry run"
    assert_selector "h1", text: /Workspace dry run #\d+/, wait: 10
    dry_run = SlackImport.last

    # The engine fills the dry run in; the plan appears from its stats.
    dry_run.update!(status: "completed", stats: slack_stats_shape(conversations: [
      slack_conversation_entry(id: "C111", name: "general"),
      slack_conversation_entry(id: "C222", name: "random", type: "private_channel")
    ]))

    visit account_slack_import_run_path(dry_run)
    click_link "Review the plan"
    assert_selector "h1", text: "Import plan", wait: 10
    assert_selector "td", text: "general"
    assert_selector "td", text: "Private channel"

    accept_confirm { click_button "Test import" }
    assert_selector "h1", text: /Workspace import #\d+/, wait: 10
    import_run = SlackImport.last

    # The run page polls: engine progress appears without navigating.
    import_run.update!(status: "running",
      stats: slack_stats_shape(overrides: { "phase" => "messages", "current" => "random" }))
    assert_selector "dd", text: "random", wait: 15
  end
end
