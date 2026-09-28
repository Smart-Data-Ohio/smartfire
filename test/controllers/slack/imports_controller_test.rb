require "test_helper"

class Slack::ImportsControllerTest < ActionDispatch::IntegrationTest
  include SlackImportUiTestHelper

  setup do
    sign_in :kevin
    @kevin = users(:kevin)
  end

  test "personal page needs workspace setup first" do
    get slack_imports_path

    assert_response :success
    assert_match(/administrator needs to set up Slack import first/, response.body)
  end

  test "personal page explains the scope and offers connect" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)

    get slack_imports_path

    assert_response :success
    assert_select "h1", "Import from Slack"
    assert_includes response.body, "direct messages, group DMs, and the private channels"
    assert_select "a", "Connect Slack"
  end

  test "personal page lists only the member's own runs" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    mine = create_slack_import!(user: @kevin, kind: "personal", status: "completed")
    theirs = create_slack_import!(user: users(:jz), kind: "personal", status: "completed")
    workspace_run = create_slack_import!(user: users(:david), kind: "workspace", status: "completed")

    get slack_imports_path

    assert_response :success
    assert_select "a[href='#{slack_import_path(mine)}']"
    assert_select "a[href='#{slack_import_path(theirs)}']", count: 0
    assert_select "a[href='#{slack_import_path(workspace_run)}']", count: 0
  end

  test "members can view their own personal runs" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    run = create_slack_import!(user: @kevin, kind: "personal", status: "running",
      stats: slack_stats_shape(overrides: { "phase" => "messages", "current" => "dm-with-jz" }))

    get slack_import_path(run)

    assert_response :success
    assert_select "h1", "Personal dry run ##{run.id}"
    assert_includes response.body, "dm-with-jz"
    assert_includes response.body, "data-controller=\"frame-poll\""
  end

  test "other members' runs 404 on the personal page" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    run = create_slack_import!(user: users(:jz), kind: "personal", status: "completed")

    get slack_import_path(run)
    assert_response :not_found

    get status_slack_import_path(run)
    assert_response :not_found

    post cancel_slack_import_path(run)
    assert_response :not_found

    post undo_slack_import_path(run)
    assert_response :not_found
  end

  test "personal status frame renders the member's own run" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    run = create_slack_import!(user: @kevin, kind: "personal", status: "running",
      stats: slack_stats_shape(overrides: { "current" => "dm-with-jz" }))

    get status_slack_import_path(run)

    assert_response :success
    assert_select "turbo-frame#slack_import_run dd", "dm-with-jz"
    assert_select "turbo-frame#slack_import_run[src]", count: 0
  end

  test "workspace runs 404 on the personal page" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    run = create_slack_import!(user: users(:david), kind: "workspace", status: "completed")

    get slack_import_path(run)
    assert_response :not_found

    post cancel_slack_import_path(run)
    assert_response :not_found
  end

  test "administrators use the admin pages for other members' runs" do
    delete session_path
    sign_in :david
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    run = create_slack_import!(user: @kevin, kind: "personal", status: "completed")

    get slack_import_path(run)
    assert_response :not_found

    get account_slack_import_run_path(run)
    assert_response :success
  end

  test "starting a preview needs workspace setup and a connection" do
    post slack_imports_path, params: { mode: "dry_run" }
    assert_redirected_to slack_imports_path
    assert_match(/administrator needs to set up/, flash[:alert])

    create_slack_workspace!(team_id: SLACK_TEAM_ID)

    post slack_imports_path, params: { mode: "dry_run" }
    assert_redirected_to slack_imports_path
    assert_match(/Connect your Slack account/, flash[:alert])
    assert_empty SlackImport.all
  end

  test "starting a preview creates a personal dry run" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connection = connect_slack!(@kevin, workspace:)

    post slack_imports_path, params: { mode: "dry_run" }

    run = SlackImport.last
    assert_redirected_to slack_import_path(run)
    assert_equal "personal", run.kind
    assert_equal "dry_run", run.mode
    assert_equal @kevin, run.user
    assert_equal connection, run.slack_connection
  end

  test "one active run per member" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@kevin, workspace:)
    create_slack_import!(workspace:, user: @kevin, kind: "personal", status: "running")

    post slack_imports_path, params: { mode: "dry_run" }

    assert_redirected_to slack_imports_path
    assert_match(/already have an import running/, flash[:alert])
    assert_equal 1, SlackImport.count
  end

  test "another member's active run queues the preview behind it" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@kevin, workspace:)
    connect_slack!(users(:jz), workspace:)
    create_slack_import!(workspace:, user: users(:jz), kind: "personal", status: "running")

    post slack_imports_path, params: { mode: "dry_run" }

    run = SlackImport.last
    assert_redirected_to slack_import_path(run)
    assert_equal "queued", run.status

    get slack_import_path(run)
    assert_includes response.body, "Queued behind another import."
  end

  test "personal import starts from the preview's checked conversations" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connection = connect_slack!(@kevin, workspace:)
    dry_run = create_slack_import!(workspace:, user: @kevin, kind: "personal",
      status: "completed",
      stats: slack_stats_shape(conversations: [
        slack_conversation_entry(id: "D111", name: "dm-with-jz", type: "im"),
        slack_conversation_entry(id: "D222", name: "dm-with-david", type: "im")
      ]))

    post slack_imports_path,
      params: { mode: "import", dry_run_id: dry_run.id, conversation_ids: [ "D111", "DBOGUS" ] }

    run = SlackImport.last
    assert_redirected_to slack_import_path(run)
    assert_equal "personal", run.kind
    assert_equal "import", run.mode
    assert_equal connection, run.slack_connection
    assert_equal [ "D111" ], run.options["conversation_ids"]
  end

  test "personal import needs a completed preview with checked conversations" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@kevin, workspace:)
    preview = create_slack_import!(workspace:, user: @kevin, kind: "personal",
      status: "completed", stats: slack_stats_shape)

    post slack_imports_path,
      params: { mode: "import", dry_run_id: preview.id + 1000, conversation_ids: [ "D111" ] }
    assert_redirected_to slack_imports_path
    assert_match(/preview first/, flash[:alert])

    post slack_imports_path,
      params: { mode: "import", dry_run_id: preview.id, conversation_ids: [] }
    assert_redirected_to slack_import_path(preview)
    assert_match(/at least one/, flash[:alert])
    assert_equal 1, SlackImport.count
  end

  test "members can cancel and undo their own runs" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    running = create_slack_import!(user: @kevin, kind: "personal", status: "running")
    import = create_slack_import!(user: @kevin, kind: "personal", mode: "import", status: "completed")

    post cancel_slack_import_path(running)
    assert_redirected_to slack_import_path(running)
    assert_equal "cancelled", running.reload.status

    post undo_slack_import_path(import)
    assert_redirected_to slack_import_path(import)
    assert_equal "undoing", import.reload.status
  end

  test "personal run page shows the plan with skip checkboxes" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    run = create_slack_import!(user: @kevin, kind: "personal", status: "completed",
      stats: slack_stats_shape(conversations: [
        slack_conversation_entry(id: "D111", name: "dm-with-jz", type: "im")
      ]))

    get slack_import_path(run)

    assert_response :success
    assert_select "input[type=checkbox][name='conversation_ids[]'][value='D111'][checked]"
    assert_select "td", "Direct message"
    assert_select "input[type=submit][value='Import checked']"
  end
end
