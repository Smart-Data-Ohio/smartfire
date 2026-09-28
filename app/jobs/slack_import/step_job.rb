class SlackImport::StepJob < ApplicationJob
  # Imports run on their own queue with a single worker, so a long import
  # never delays push notifications on the default worker.
  queue_as :slack_import

  def perform(import_id)
    run = SlackImport.find_by(id: import_id)
    return if run.nil?

    if run.queued?
      # A lost claim exits without re-enqueueing: the stamp is cleared so
      # the next finish or sweeper tick enqueues a fresh job.
      unless SlackImport.claim_running!(run.id)
        run.clear_pending_step_job!
        return
      end

      run.reload
    end
    return unless run.running?

    unless run.slack_connection&.connected?
      run.mark_failed!("Slack connection is missing or disconnected")
      return
    end

    # The lease marks the run busy while this step executes, so a cancel
    # that lands mid-step still blocks new runs and undos until the
    # in-flight step stops writing. A lost race simply returns: the
    # holder continues the chain, or, if it crashed, the sweeper
    # re-enqueues once the heartbeat goes stale — which always
    # outlasts the lease, so the run can never strand with no job.
    lease_token = run.acquire_step_lease!("running")
    return unless lease_token

    # The lease releases (in the ensure) before the next job is
    # enqueued below, so the next job acquires immediately instead of
    # failing to acquire on another worker and stalling the run until
    # the sweeper re-enqueues it.
    outcome = nil
    begin
      outcome = SlackImport::Runner.new(run).step!
    ensure
      run.release_step_lease!(lease_token)
    end
    self.class.perform_later(run.id) if outcome == :continue
    # A stopped step's run finished elsewhere (cancelled mid-step): hand
    # off promptly instead of waiting for the sweeper.
    SlackImport.kick_next_queued! if outcome == :stopped
  rescue Slack::Client::RateLimited => error
    # Slack asked for a pause: heartbeat first so the sweeper does not pile
    # on, then resume after Retry-After. Delayed jobs drain via bin/periodic.
    if run&.reload&.running?
      run.update_columns(heartbeat_at: Time.current)
      self.class.set(wait: error.retry_after).perform_later(import_id)
    end
  rescue Slack::Client::ScopeError => error
    fail_run(run, error.message)
  rescue Slack::Client::AuthError => error
    fail_run(run, error.message)
    run&.slack_connection&.update(disconnected_reason: error.message)
  rescue Slack::Client::Error => error
    fail_run(run, error.message)
  rescue StandardError => error
    raise if ApplicationJob::TRANSIENT_ERRORS.any? { |kind| error.is_a?(kind) }

    Rails.error.report(error, handled: true, context: { slack_import_id: import_id })
    fail_run(run, "#{error.class}: #{error.message}")
  end

  private
    # Only a still-running run fails: a run cancelled mid-step keeps its
    # cancelled status instead of flipping to failed.
    def fail_run(run, message)
      return if run.nil?

      run.reload
      run.mark_failed!(message) if run.running?
    end
end
