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

  test "claim_running! refuses while another run is undoing" do
    undoing = start_run
    undoing.update!(status: "undoing", heartbeat_at: Time.current)
    queued = start_run

    assert_not SlackImport.claim_running!(queued.id)
    assert queued.reload.queued?
  end

  test "a step job that cannot claim its queued run exits without re-enqueueing" do
    first = start_run
    second = start_run
    first.update!(status: "running", heartbeat_at: Time.current)
    clear_enqueued_jobs

    SlackImport::StepJob.perform_now(second.id)

    assert second.reload.queued?
    assert_empty enqueued_jobs
    assert_not second.reload.step_job_pending?
  end

  test "undo! refuses with no status change while another run is queued, running or undoing" do
    %w[ queued running undoing ].each do |status|
      import = start_run
      import.update!(status: "completed", finished_at: Time.current)
      finished_at = import.finished_at
      other = start_run
      other.update!(status:, heartbeat_at: Time.current)

      assert_no_enqueued_jobs do
        assert_not import.undo!, "undo should be blocked by a #{status} run"
      end
      assert_equal "completed", import.reload.status
      assert_equal finished_at, import.finished_at
      assert_equal "Another import is queued or running. Wait for it to finish, then undo.",
        import.undo_blocked_reason

      other.destroy!
      import.destroy!
    end
  end

  test "undo_blocked_reason is nil when nothing else is active" do
    run = start_run
    run.update!(status: "completed", finished_at: Time.current)

    assert_nil run.undo_blocked_reason
  end

  test "failed and cancelled runs kick the next queued run" do
    %i[ fail cancel ].each do |finisher|
      first = start_run
      second = start_run
      first.update!(status: "running", heartbeat_at: Time.current)
      second.clear_pending_step_job!
      clear_enqueued_jobs

      finisher == :fail ? first.mark_failed!("boom") : first.cancel!

      assert_enqueued_with(job: SlackImport::StepJob, args: [ second.id ])
      assert second.reload.step_job_pending?

      first.destroy!
      second.destroy!
    end
  end

  test "step_finishing does not overwrite a cancelled run" do
    run = start_run
    run.update!(status: "running", state: { "phase" => "finishing" }, heartbeat_at: Time.current)
    run.update!(status: "cancelled", finished_at: Time.current)

    outcome = SlackImport::Runner.new(run).send(:step_finishing)

    assert_equal :stopped, outcome
    assert run.reload.cancelled?
  end

  test "step_finishing completes a running run and kicks the next queued one" do
    first = start_run
    second = start_run
    first.update!(status: "running", state: { "phase" => "finishing" }, heartbeat_at: Time.current)
    second.clear_pending_step_job!
    clear_enqueued_jobs

    outcome = SlackImport::Runner.new(first).send(:step_finishing)

    assert_equal :done, outcome
    assert first.reload.completed?
    assert_equal "done", first.stats["phase"]
    assert_enqueued_with(job: SlackImport::StepJob, args: [ second.id ])
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

  test "sweep starts the oldest queued run only when nothing runs or undoes" do
    first = start_run
    second = start_run
    first.clear_pending_step_job!
    second.clear_pending_step_job!
    clear_enqueued_jobs

    SlackImport.sweep_stalled!

    assert_enqueued_with(job: SlackImport::StepJob, args: [ first.id ])
    assert_equal 1, enqueued_jobs.count { |job| job[:job] == SlackImport::StepJob }

    clear_enqueued_jobs
    first.update!(status: "running", heartbeat_at: Time.current)
    SlackImport.sweep_stalled!

    assert_empty enqueued_jobs.select { |job| job[:job] == SlackImport::StepJob }

    clear_enqueued_jobs
    first.update!(status: "undoing", heartbeat_at: Time.current)
    SlackImport.sweep_stalled!

    assert_empty enqueued_jobs.select { |job| job[:job] == SlackImport::StepJob }
  end

  test "sweep never enqueues a second job for a run with one pending" do
    run = start_run
    clear_enqueued_jobs

    SlackImport.sweep_stalled!

    assert_empty enqueued_jobs.select { |job| job[:job] == SlackImport::StepJob }

    run.update!(state: { "enqueued_at" => 10.minutes.ago.iso8601(6) })
    SlackImport.sweep_stalled!

    assert_enqueued_with(job: SlackImport::StepJob, args: [ run.id ])
  end

  test "sweep re-enqueues stalled undoing runs" do
    run = start_run
    run.update!(status: "undoing", heartbeat_at: 10.minutes.ago)
    clear_enqueued_jobs

    SlackImport.sweep_stalled!

    assert_enqueued_with(job: SlackImport::UndoJob, args: [ run.id ])
  end
end
