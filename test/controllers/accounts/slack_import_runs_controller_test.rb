require "test_helper"

class Accounts::SlackImportRunsControllerTest < ActionDispatch::IntegrationTest
  include SlackImportUiTestHelper

  setup do
    sign_in :david
    @david = users(:david)
  end

  test "run pages are admin-only" do
    run = create_slack_import!(user: @david)
    delete session_path
    sign_in :kevin

    get account_slack_import_runs_path
    assert_response :forbidden

    get account_slack_import_run_path(run)
    assert_response :forbidden

    get status_account_slack_import_run_path(run)
    assert_response :forbidden

    get plan_account_slack_import_run_path(run)
    assert_response :forbidden

    post account_slack_import_runs_path
    assert_response :forbidden
  end

  test "run list shows every run newest first" do
    older = create_slack_import!(user: @david, status: "completed", created_at: 2.days.ago)
    personal = create_slack_import!(user: users(:kevin), kind: "personal", status: "running", created_at: 1.day.ago)
    newer = create_slack_import!(user: @david, status: "queued", created_at: 1.hour.ago)

    get account_slack_import_runs_path

    assert_response :success
    ids = response.body.scan(%r{/account/slack_import/runs/(\d+)}).flatten.map(&:to_i)
    assert_equal [ newer.id, personal.id, older.id ], ids
    assert_select "td", "personal"
    assert_select "td", "Kevin"
  end

  test "starting a dry run needs a connected account" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)

    post account_slack_import_runs_path

    assert_redirected_to account_slack_import_path
    assert_match(/Connect your Slack account/, flash[:alert])
    assert_empty SlackImport.all
  end

  test "starting a dry run is blocked while another run is active" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david, workspace:)
    create_slack_import!(workspace:, user: users(:kevin), kind: "personal", status: "running")

    post account_slack_import_runs_path

    assert_redirected_to account_slack_import_runs_path
    assert_match(/already running/, flash[:alert])
    assert_equal 1, SlackImport.count
  end

  test "starting a dry run creates a workspace dry run with the options" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connection = connect_slack!(@david, workspace:)

    post account_slack_import_runs_path,
      params: { include_private: "0", oldest: "2026-01-01", latest: "2026-06-01" }

    run = SlackImport.last
    assert_redirected_to account_slack_import_run_path(run)
    assert_equal "workspace", run.kind
    assert_equal "dry_run", run.mode
    assert_equal "queued", run.status
    assert_equal @david, run.user
    assert_equal connection, run.slack_connection
    assert_equal({ "include_private" => false, "oldest" => "2026-01-01", "latest" => "2026-06-01" }, run.options)
    assert_equal "slack.import.start", AuditLog.last.action
  end

  test "starting a dry run defaults to including private channels" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david, workspace:)

    post account_slack_import_runs_path, params: { include_private: "1" }

    assert_equal true, SlackImport.last.options["include_private"]
  end

  test "run page shows status, counts, timestamps, and issues with pagination" do
    run = create_slack_import!(user: @david, status: "running",
      stats: slack_stats_shape(overrides: { "phase" => "messages", "current" => "general" }))
    52.times do |index|
      SlackImport::Issue.create!(slack_import: run, level: "warning",
        slack_ref: "C111", message: "issue #{index}", created_at: Time.current)
    end

    get account_slack_import_run_path(run)

    assert_response :success
    assert_select "h1", "Workspace dry run ##{run.id}"
    assert_includes response.body, "messages"
    assert_includes response.body, "general"
    assert_includes response.body, "7 found"
    assert_includes response.body, "100 messages"
    assert_includes response.body, "42"
    assert_includes response.body, "issue 0"
    assert_select "a", "Older issues"

    get account_slack_import_run_path(run, page: last_next_param)
    assert_includes response.body, "issue 51"
  end

  test "run page polls while active and stops when finished" do
    active = create_slack_import!(user: @david, status: "running")
    finished = create_slack_import!(user: @david, status: "completed")

    get account_slack_import_run_path(active)
    assert_includes response.body, "data-controller=\"frame-poll\""
    assert_select "turbo-frame#slack_import_run[src=?]", status_account_slack_import_run_path(active)

    get account_slack_import_run_path(finished)
    assert_not_includes response.body, "data-controller=\"frame-poll\""
  end

  test "status frame renders the run without polling itself" do
    run = create_slack_import!(user: @david, status: "running",
      stats: slack_stats_shape(overrides: { "current" => "general" }))

    get status_account_slack_import_run_path(run)

    assert_response :success
    assert_select "turbo-frame#slack_import_run", count: 1
    assert_select "turbo-frame#slack_import_run dd", "general"
    assert_select "turbo-frame#slack_import_run[data-controller]", count: 0
    assert_select "turbo-frame#slack_import_run[src]", count: 0
  end

  test "status frame marks a finished run so polling stops" do
    run = create_slack_import!(user: @david, status: "completed")

    get status_account_slack_import_run_path(run)

    assert_response :success
    assert_select "[data-frame-poll-finished]"
  end

  test "run page shows queued-behind while another run is active" do
    workspace = SlackWorkspace.current || create_slack_workspace!
    create_slack_import!(workspace:, user: users(:kevin), status: "running")
    queued = create_slack_import!(workspace:, user: @david, status: "queued")

    get account_slack_import_run_path(queued)

    assert_includes response.body, "Queued behind another import."
  end

  test "administrators can view personal runs from the admin page" do
    run = create_slack_import!(user: users(:kevin), kind: "personal", status: "completed")

    get account_slack_import_run_path(run)

    assert_response :success
    assert_select "h1", "Personal dry run ##{run.id}"
  end

  test "plan renders conversations, targets, and samples" do
    room = rooms(:watercooler)
    conversations = [
      slack_conversation_entry(id: "C111", name: "general", type: "public_channel"),
      slack_conversation_entry(id: "G222", name: "secret", type: "private_channel", archived: true,
        target: { "action" => "merge", "room_id" => room.id, "room_name" => room.name })
    ]
    run = create_slack_import!(user: @david, status: "completed",
      stats: slack_stats_shape(conversations:))
    create_slack_workspace!(team_id: SLACK_TEAM_ID)

    get plan_account_slack_import_run_path(run)

    assert_response :success
    assert_select "td", "general"
    assert_select "td", "Private channel"
    assert_select "td", "Yes"
    assert_select "select[name='room_targets[C111]'] option[selected]", "New room"
    assert_select "select[name='room_targets[G222]'] option[selected]", room.name
    assert_select "td", "hello *world*"
    assert_includes response.body, "hello <strong>world</strong>"
  end

  test "plan escapes Slack text and sample markdown" do
    conversations = [ slack_conversation_entry(name: "<script>alert(1)</script>") ]
    samples = [ { "conversation" => "x", "slack_text" => "<img src=x onerror=alert(1)>",
      "markdown" => "<script>alert(2)</script> hi" } ]
    run = create_slack_import!(user: @david, status: "completed",
      stats: slack_stats_shape(conversations:, samples:))

    get plan_account_slack_import_run_path(run)

    assert_response :success
    assert_not_includes response.body, "<script>alert(1)</script>"
    assert_includes response.body, "&lt;script&gt;alert(1)&lt;/script&gt;"
    assert_not_includes response.body, "<img src=x onerror=alert(1)>"
    assert_not_includes response.body, "<script>alert(2)</script>"
    assert_includes response.body, "hi"
  end

  test "plan needs a completed dry run" do
    running = create_slack_import!(user: @david, status: "running")
    import = create_slack_import!(user: @david, mode: "import", status: "completed")

    get plan_account_slack_import_run_path(running)
    assert_redirected_to account_slack_import_run_path(running)

    get plan_account_slack_import_run_path(import)
    assert_redirected_to account_slack_import_run_path(import)
  end

  test "test import starts with checked conversations and a recent default" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connection = connect_slack!(@david, workspace:)
    room = rooms(:watercooler)
    dry_run = create_slack_import!(workspace:, user: @david, status: "completed",
      options: { "include_private" => true },
      stats: slack_stats_shape(conversations: [
        slack_conversation_entry(id: "C111"), slack_conversation_entry(id: "C222")
      ]))

    travel_to Time.utc(2026, 9, 28, 12) do
      post start_import_account_slack_import_run_path(dry_run), params: {
        preset: "test", conversation_ids: [ "C111" ],
        room_targets: { "C111" => "new" }
      }
    end

    run = SlackImport.last
    assert_redirected_to account_slack_import_run_path(run)
    assert_equal "import", run.mode
    assert_equal connection, run.slack_connection
    assert_equal [ "C111" ], run.options["conversation_ids"]
    assert_equal "2026-09-14", run.options["oldest"]
    assert_nil run.options["latest"]
    assert_equal true, run.options["include_private"]
    assert_equal({ "C111" => "new" }, run.options["room_targets"])
  end

  test "full import starts with no date bounds" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david, workspace:)
    room = rooms(:watercooler)
    dry_run = create_slack_import!(workspace:, user: @david, status: "completed",
      stats: slack_stats_shape)

    post start_import_account_slack_import_run_path(dry_run), params: {
      preset: "full", conversation_ids: [ "C111" ],
      room_targets: { "C111" => room.id.to_s },
      oldest: "2026-01-01", latest: "2026-02-01"
    }

    options = SlackImport.last.options
    assert_equal [ "C111" ], options["conversation_ids"]
    assert_nil options["oldest"]
    assert_nil options["latest"]
    assert_equal({ "C111" => room.id }, options["room_targets"])
  end

  test "starting an import with nothing checked is rejected" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david, workspace:)
    dry_run = create_slack_import!(workspace:, user: @david, status: "completed",
      stats: slack_stats_shape)

    post start_import_account_slack_import_run_path(dry_run),
      params: { preset: "test", conversation_ids: [] }

    assert_redirected_to plan_account_slack_import_run_path(dry_run)
    assert_match(/at least one/, flash[:alert])
    assert_equal 1, SlackImport.count
  end

  test "starting an import is blocked without a connection or with an active run" do
    create_slack_workspace!(team_id: SLACK_TEAM_ID)
    dry_run = create_slack_import!(user: @david, status: "completed", stats: slack_stats_shape)

    post start_import_account_slack_import_run_path(dry_run),
      params: { preset: "test", conversation_ids: [ "C111" ] }
    assert_redirected_to plan_account_slack_import_run_path(dry_run)
    assert_match(/Connect your Slack account/, flash[:alert])

    connect_slack!(@david)
    create_slack_import!(user: users(:kevin), kind: "personal", status: "running")

    post start_import_account_slack_import_run_path(dry_run),
      params: { preset: "test", conversation_ids: [ "C111" ] }
    assert_redirected_to plan_account_slack_import_run_path(dry_run)
    assert_match(/already running/, flash[:alert])
  end

  test "catch-up repeats the import's conversations and targets" do
    workspace = create_slack_workspace!(team_id: SLACK_TEAM_ID)
    connect_slack!(@david, workspace:)
    room = rooms(:watercooler)
    import = create_slack_import!(workspace:, user: @david, mode: "import", status: "completed",
      options: { "conversation_ids" => [ "C111" ], "room_targets" => { "C111" => room.id },
        "include_private" => false, "oldest" => "2026-09-14" })

    post catch_up_account_slack_import_run_path(import)

    run = SlackImport.last
    assert_redirected_to account_slack_import_run_path(run)
    assert_equal "import", run.mode
    assert_equal [ "C111" ], run.options["conversation_ids"]
    assert_equal({ "C111" => room.id }, run.options["room_targets"])
    assert_equal false, run.options["include_private"]
    assert_nil run.options["oldest"]
  end

  test "catch-up needs a completed import" do
    dry_run = create_slack_import!(user: @david, status: "completed")

    post catch_up_account_slack_import_run_path(dry_run)

    assert_redirected_to account_slack_import_run_path(dry_run)
    assert_equal 1, SlackImport.count
  end

  test "cancel and undo act on workspace runs" do
    running = create_slack_import!(user: @david, status: "running")
    import = create_slack_import!(user: @david, mode: "import", status: "completed")

    post cancel_account_slack_import_run_path(running)
    assert_redirected_to account_slack_import_run_path(running)
    assert_equal "cancelled", running.reload.status
    assert_equal "slack.import.cancel", AuditLog.last.action

    post undo_account_slack_import_run_path(import)
    assert_redirected_to account_slack_import_run_path(import)
    assert_equal "undoing", import.reload.status
    assert_equal "slack.import.undo", AuditLog.last.action
  end

  test "administrators may cancel and undo personal runs" do
    running = create_slack_import!(user: users(:kevin), kind: "personal", status: "running")
    import = create_slack_import!(user: users(:kevin), kind: "personal", mode: "import", status: "completed")

    post cancel_account_slack_import_run_path(running)
    assert_equal "cancelled", running.reload.status

    post undo_account_slack_import_run_path(import)
    assert_equal "undoing", import.reload.status
  end

  test "cancel and undo refuse finished and non-undoable runs" do
    finished = create_slack_import!(user: @david, status: "completed")
    dry_run = create_slack_import!(user: @david, status: "completed")

    post cancel_account_slack_import_run_path(finished)
    assert_match(/already finished/, flash[:alert])

    post undo_account_slack_import_run_path(dry_run)
    assert_match(/cannot be undone/, flash[:alert])
  end

  private
    def last_next_param
      response.body[%r{page=([^"&]+)[^>]*>Older issues}, 1]
    end
end
