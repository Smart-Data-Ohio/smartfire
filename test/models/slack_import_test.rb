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

  test "claim_running! refuses while a cancelled run holds a fresh step lease" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    lease_token = run.acquire_step_lease!("running")
    assert lease_token
    other = start_run
    assert_not SlackImport.claim_running!(other.id)

    # The cancel flips the status while the step is still mid-flight: the
    # fresh lease keeps blocking until the step ends and clears it.
    run.cancel!

    assert run.reload.cancelled?
    assert run.step_lease_fresh?
    assert_not SlackImport.claim_running!(other.id)
    assert other.reload.queued?

    run.release_step_lease!(lease_token)

    assert_not run.reload.step_lease_fresh?
    assert SlackImport.claim_running!(other.id)
    assert_equal "running", other.reload.status
  end

  test "a stale step lease no longer blocks claims" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    run.acquire_step_lease!("running")
    run.cancel!
    other = start_run

    travel SlackImport::STALE_HEARTBEAT + 1.minute do
      assert SlackImport.claim_running!(other.id)
    end

    assert_equal "running", other.reload.status
  end

  test "a second acquire fails while a fresh lease is held" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    token = run.acquire_step_lease!("running")
    assert token.is_a?(String)

    # A second job process sees the same row through its own instance.
    other_view = SlackImport.find(run.id)

    assert_equal false, other_view.acquire_step_lease!("running")
    assert_equal token, run.reload.state["step_lease_token"]
    assert run.step_lease_fresh?
  end

  test "a stale lease can be taken over without losing saved progress" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    first_token = run.acquire_step_lease!("running")
    run.update!(state: run.state.merge("convo_index" => 3))

    second_token = travel(SlackImport::STALE_HEARTBEAT + 1.minute) do
      SlackImport.find(run.id).acquire_step_lease!("running")
    end

    assert second_token.is_a?(String)
    assert_not_equal first_token, second_token
    assert_equal second_token, run.reload.state["step_lease_token"]
    assert_equal 3, run.state["convo_index"]
    # The previous holder's token no longer releases anything.
    assert_equal false, run.release_step_lease!(first_token)
    assert run.reload.step_lease_fresh?
  end

  test "releasing with the wrong token keeps the lease" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    token = run.acquire_step_lease!("running")

    assert_equal false, run.release_step_lease!("bogus-token")
    assert run.reload.step_lease_fresh?
    assert_equal token, run.state["step_lease_token"]

    assert run.release_step_lease!(token)
    assert_not run.reload.step_lease_fresh?
    assert_nil run.state["step_started_at"]
    assert_nil run.state["step_lease_token"]
  end

  test "a step that raises still releases its lease" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    SlackImport::Runner.any_instance.stubs(:step!).raises(RuntimeError, "boom")

    SlackImport::StepJob.perform_now(run.id)

    run.reload
    assert_equal "failed", run.status
    assert_not run.step_lease_fresh?
    assert_nil run.state["step_started_at"]
    assert_nil run.state["step_lease_token"]
  end

  test "another job's lease write is not mistaken for progress on conflict" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current,
      state: { "phase" => "users" })
    token = run.acquire_step_lease!("running")
    runner = SlackImport::Runner.new(run)
    runner.send(:transition_to, "conversations")

    # Another job takes over the lease without saving progress: only the
    # lease keys differ on disk when this step hits its conflict.
    run.release_step_lease!(token)
    assert SlackImport.find(run.id).acquire_step_lease!("running")
    runner.stubs(:step_conversations).raises(ActiveRecord::RecordNotUnique.new("conflict"))

    assert_raises(ActiveRecord::RecordNotUnique) { runner.step! }
  end

  test "a lease-looking state on a completed run does not block claims" do
    # Completed runs never hold a lease (finishing strips it), so the
    # blocking scopes only read leases off statuses that still can —
    # and a stray stamp here blocks nothing.
    finished = start_run
    finished.update!(status: "completed", started_at: 1.hour.ago, finished_at: Time.current,
      state: finished.state.merge("step_started_at" => Time.current.iso8601(6),
        "step_lease_token" => "stray"))
    queued = start_run

    assert SlackImport.claim_running!(queued.id)
    assert_equal "running", queued.reload.status
  end

  test "step lease stamps are written in UTC even under a user time zone" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)

    Time.use_zone("Tokyo") do
      assert run.acquire_step_lease!("running")
    end

    assert_match(/Z\z/, run.state["step_started_at"])
  end

  test "fresh leases block and stale leases pass under user time zones" do
    %w[ Tokyo America/New_York ].each do |zone|
      import = start_run
      import.update!(status: "completed", started_at: 1.hour.ago, finished_at: Time.current)
      undo_candidate = start_run
      undo_candidate.update!(status: "completed", started_at: 1.hour.ago, finished_at: Time.current)
      busy = start_run
      busy.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
      # Jobs acquire in UTC; the reads below run under the user's zone,
      # like controller requests do through SetTimeZone.
      busy.acquire_step_lease!("running")
      busy.cancel!

      Time.use_zone(zone) do
        assert SlackImport.with_fresh_lease.where(id: busy.id).exists?,
          "fresh lease should block under #{zone}"
        assert_equal "Another import is still finishing. Wait for it to finish, then undo.",
          import.undo_blocked_reason
        # The pre-check is stubbed out so only the atomic UPDATE can
        # refuse the undo.
        import.stubs(:undo_blocked_reason).returns(nil)
        assert_no_enqueued_jobs do
          assert_not import.undo!, "undo claim should refuse under #{zone}"
        end

        fresh_queued = start_run
        fresh_queued.clear_pending_step_job!
        assert_no_enqueued_jobs do
          SlackImport.kick_next_queued!
        end
        assert fresh_queued.reload.queued?, "kick should wait under #{zone}"
        fresh_queued.destroy!
      end

      # The step ended long ago; only a stale stamp is left behind.
      busy.update!(state: busy.state.merge("step_started_at" => 6.minutes.ago.utc.iso8601(6)))

      Time.use_zone(zone) do
        assert_not SlackImport.with_fresh_lease.where(id: busy.id).exists?,
          "stale lease should pass under #{zone}"
        assert_nil undo_candidate.undo_blocked_reason
        assert_enqueued_with(job: SlackImport::UndoJob) do
          assert undo_candidate.undo!, "undo should proceed under #{zone}"
        end
      end
      # The now-undoing run would block the kick below by status.
      undo_candidate.destroy!

      stale_queued = start_run
      stale_queued.clear_pending_step_job!
      clear_enqueued_jobs
      Time.use_zone(zone) do
        SlackImport.kick_next_queued!
      end
      assert_enqueued_with(job: SlackImport::StepJob, args: [ stale_queued.id ])

      import.destroy!
      busy.destroy!
      stale_queued.destroy!
    end
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

  test "the undo claim itself refuses a queued run that slips in after the pre-check" do
    import = start_run
    import.update!(status: "completed", started_at: 1.hour.ago, finished_at: Time.current)
    assert_nil import.undo_blocked_reason

    # A new run starts after the pre-check ran. The pre-check is stubbed
    # out so only the atomic UPDATE can refuse the undo.
    import.stubs(:undo_blocked_reason).returns(nil)
    queued = start_run

    assert_no_enqueued_jobs do
      assert_not import.undo!
    end
    assert_equal "completed", import.reload.status
    assert queued.reload.queued?
  end

  test "the undo claim itself refuses a fresh lease held outside an active status" do
    import = start_run
    import.update!(status: "completed", started_at: 1.hour.ago, finished_at: Time.current)
    assert_nil import.undo_blocked_reason

    busy = start_run
    busy.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    busy.acquire_step_lease!("running")
    busy.cancel!

    # The pre-check is stubbed out so only the atomic UPDATE can refuse
    # the undo; the cancelled run blocks by lease, not by status.
    import.stubs(:undo_blocked_reason).returns(nil)

    assert_no_enqueued_jobs do
      assert_not import.undo!
    end
    assert_equal "completed", import.reload.status
  end

  test "undo_blocked_reason is nil when nothing else is active" do
    run = start_run
    run.update!(status: "completed", finished_at: Time.current)

    assert_nil run.undo_blocked_reason
  end

  test "undo waits while the run itself holds a fresh step lease" do
    run = start_run
    run.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    lease_token = run.acquire_step_lease!("running")
    run.cancel!

    assert_equal "This import is still finishing. Wait for it to finish, then undo.",
      run.undo_blocked_reason
    assert_no_enqueued_jobs do
      assert_not run.undo!
    end
    assert_equal "cancelled", run.reload.status

    run.release_step_lease!(lease_token)

    assert_nil run.undo_blocked_reason
    assert_enqueued_with(job: SlackImport::UndoJob) do
      assert run.undo!
    end
  end

  test "undo waits while another run holds a fresh step lease" do
    import = start_run
    import.update!(status: "completed", started_at: 1.hour.ago, finished_at: Time.current)
    busy = start_run
    busy.update!(status: "running", started_at: Time.current, heartbeat_at: Time.current)
    busy_token = busy.acquire_step_lease!("running")
    busy.cancel!

    assert_equal "Another import is still finishing. Wait for it to finish, then undo.",
      import.undo_blocked_reason
    assert_not import.undo!
    assert_equal "completed", import.reload.status

    busy.release_step_lease!(busy_token)

    assert_nil import.undo_blocked_reason
    assert import.undo!
  end

  test "the later-overlap answer refreshes after reload" do
    first = start_run
    first.update!(status: "completed", started_at: 2.hours.ago, finished_at: 1.hour.ago,
      stats: { "conversations" => [ { "id" => "CCHAN", "target" => { "action" => "create" } } ] })
    assert_nil first.undo_blocked_reason

    later = start_run
    later.update!(status: "completed", started_at: 1.hour.ago, finished_at: Time.current,
      stats: { "conversations" => [ { "id" => "CCHAN", "target" => { "action" => "merge" } } ] })

    first.reload

    assert_equal "A later import (##{later.id}) also imported some of these conversations; undo that one first.",
      first.undo_blocked_reason
    assert_not first.undoable?
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
