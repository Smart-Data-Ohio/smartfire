class SlackImport::StepJob < ApplicationJob
  # Imports run on their own queue with a single worker, so a long import
  # never delays push notifications on the default worker.
  queue_as :slack_import

  def perform(import_id)
    run = SlackImport.find_by(id: import_id)
    return if run.nil?

    if run.queued?
      return unless SlackImport.claim_running!(run.id)

      run.reload
    end
    return unless run.running?

    unless run.slack_connection&.connected?
      run.mark_failed!("Slack connection is missing or disconnected")
      return
    end

    outcome = SlackImport::Runner.new(run).step!
    self.class.perform_later(run.id) if outcome == :continue
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
