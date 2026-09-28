class SlackImport::UndoJob < ApplicationJob
  queue_as :slack_import

  def perform(import_id)
    run = SlackImport.find_by(id: import_id)
    return if run.nil? || !run.undoing?

    # A lost race simply returns: the lease holder continues the undo,
    # or, if it crashed, the sweeper re-enqueues once the heartbeat
    # goes stale — which always outlasts the lease, so the run can
    # never strand with no job.
    lease_token = run.acquire_step_lease!("undoing")
    return unless lease_token

    # The lease releases (in the ensure) before the next job is
    # enqueued below, so the next job acquires immediately instead of
    # failing to acquire on another worker and stalling the run until
    # the sweeper re-enqueues it.
    outcome = nil
    begin
      outcome = SlackImport::Undoer.new(run).step!
    ensure
      run.release_step_lease!(lease_token)
    end
    self.class.perform_later(run.id) if outcome == :continue
    SlackImport.kick_next_queued! if outcome == :stopped
  end
end
