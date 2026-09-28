require "test_helper"

class SlackImportTest < ActiveSupport::TestCase
  include SlackImportTestHelper

  setup do
    @workspace = create_slack_workspace!
    @connection = create_slack_connection!(workspace: @workspace, user: users(:david))
  end

  def start_run(**overrides)
    defaults = { workspace: @workspace, user: users(:david), connection: @connection,
      kind: "workspace", mode: "import", options: {} }
    SlackImport.start!(**defaults.merge(overrides))
  end

  test "start! creates a queued run with normalized options and enqueues it" do
    assert_enqueued_with(job: SlackImport::StepJob) do
      @run = start_run(options: { "conversation_ids" => [ "C1", "", nil ],
        "oldest" => "2023-11-14T22:13:30Z", "include_private" => false,
        "room_targets" => { "C1" => "skip" }, "bogus" => 1 })
    end

    assert @run.queued?
    assert_equal({ "conversation_ids" => [ "C1" ], "oldest" => "2023-11-14T22:13:30.000000Z",
      "latest" => nil, "include_private" => false, "room_targets" => { "C1" => "skip" } },
      @run.options)
  end

  test "start! defaults to including private channels" do
    assert_equal true, start_run.options["include_private"]
  end

  test "start! rejects unparseable date bounds" do
    assert_raises(ArgumentError) { start_run(options: { "oldest" => "not a date" }) }
  end

  test "cancel! stops queued and running runs at their boundary" do
    run = start_run
    assert run.cancellable?
    assert run.cancel!
    assert run.cancelled?
    assert_not_nil run.finished_at
    assert_not run.cancellable?
    assert_not run.cancel!
  end

  test "undo! enqueues the undo job and resets progress tracking" do
    run = start_run
    run.update!(status: "completed", finished_at: Time.current)

    assert run.undoable?
    assert_enqueued_with(job: SlackImport::UndoJob) do
      assert run.undo!
    end

    assert run.undoing?
    assert_equal "undo", run.state["phase"]
    assert_equal "undo", run.stats["phase"]
    assert_not_nil run.heartbeat_at
    assert_nil run.finished_at
  end

  test "undo! refuses dry runs and active runs" do
    dry = start_run(mode: "dry_run")
    dry.update!(status: "completed")
    assert_not dry.undoable?
    assert_not dry.undo!

    active = start_run
    assert_not active.undoable?
    assert_not active.undo!
  end

  test "claim_running! lets a single queued run through while none runs" do
    first = start_run
    second = start_run

    assert SlackImport.claim_running!(first.id)
    assert_equal "running", first.reload.status
    assert_not SlackImport.claim_running!(second.id)
    assert second.reload.queued?
    assert_not SlackImport.claim_running!(first.id)
  end

  test "record_issue! caps issues with a suppression notice" do
    run = start_run

    SlackImport::ISSUE_CAP.times { |index| run.record_issue!("warning", nil, "issue #{index}") }
    run.record_issue!("warning", nil, "one too many")
    run.record_issue!("warning", nil, "never stored")

    assert_equal SlackImport::ISSUE_CAP + 1, run.issues.count
    assert_includes run.issues.last.message, "suppressed"
  end

  test "sweep re-enqueues stale running runs" do
    stale = start_run
    stale.update!(status: "running", heartbeat_at: 10.minutes.ago)
    fresh = start_run
    fresh.update!(status: "running", heartbeat_at: Time.current)
    clear_enqueued_jobs

    SlackImport.sweep_stalled!

    step_jobs = enqueued_jobs.select { |job| job[:job] == SlackImport::StepJob }
    assert_equal [ stale.id ], step_jobs.map { |job| job[:args].first }
  end

  test "sweep starts the oldest queued run only when nothing runs" do
    first = start_run
    second = start_run
    clear_enqueued_jobs

    SlackImport.sweep_stalled!

    assert_enqueued_with(job: SlackImport::StepJob, args: [ first.id ])
    assert_equal 1, enqueued_jobs.count { |job| job[:job] == SlackImport::StepJob }

    clear_enqueued_jobs
    first.update!(status: "running", heartbeat_at: Time.current)
    SlackImport.sweep_stalled!

    assert_empty enqueued_jobs.select { |job| job[:job] == SlackImport::StepJob }
  end

  test "sweep re-enqueues stalled undoing runs" do
    run = start_run
    run.update!(status: "undoing", heartbeat_at: 10.minutes.ago)
    clear_enqueued_jobs

    SlackImport.sweep_stalled!

    assert_enqueued_with(job: SlackImport::UndoJob, args: [ run.id ])
  end
end
