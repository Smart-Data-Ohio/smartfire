class Agent::EventWebhookJob < ApplicationJob
  # One POST attempt per execution, with backoff between attempts. The
  # row's own attempt counter governs exhaustion (not executions) so a
  # retried execution and a repeated perform_now observe the same cap.
  MAX_ATTEMPTS = 5

  retry_on StandardError, wait: :polynomially_longer, attempts: MAX_ATTEMPTS

  # Posts one event row to the agent's webhook. Only a 2xx answer counts
  # as delivered. Transport failures and retryable answers (429, 408,
  # 5xx) record an attempt and retry with backoff up to MAX_ATTEMPTS,
  # then the row is marked failed with the last error; a retryable
  # answer carrying Retry-After waits out the endpoint's delay instead
  # of the default backoff. Guard refusals, unresolvable hosts,
  # payloads that can no longer be built, and other 4xx answers fail
  # fast without retrying. Never raises for delivery failures once
  # recorded.
  #
  # The attempt argument is the webhook_attempts value the job was
  # scheduled for. The claim below increments only when the row still
  # holds that value, so a duplicate enqueue (a sweeper pass racing the
  # scheduled retry) finds its attempt number stale and exits without
  # posting or burning an attempt. Jobs enqueued before the attempt
  # argument existed pass nil and claim whatever the row currently
  # holds.
  def perform(event_id, attempt = nil)
    event = AgentEvent.find_by(id: event_id)
    return unless event
    return if event.webhook_status == "delivered"

    webhook = event.agent.user.webhook
    if webhook.nil?
      event.update!(webhook_status: "none")
      return
    end

    attempt = event.webhook_attempts.to_i if attempt.nil?
    claimed = AgentEvent.where(id: event.id, webhook_attempts: attempt, webhook_status: "pending")
      .update_all(webhook_attempts: attempt + 1, webhook_next_attempt_at: Time.current) == 1
    return unless claimed

    attempts = attempt + 1
    # Reload so later update! calls see the claimed row as their
    # baseline; without it the fail-fast revert below looks like a
    # no-op to dirty tracking (0 -> 1 -> 0) and never writes.
    event.reload

    begin
      Agent::Delivery.post_event_webhook!(webhook, event, agent: event.agent)
    rescue Agent::Delivery::UndeliverableWebhook, RestrictedHTTP::Violation, Surfguard::Unresolvable, URI::InvalidURIError,
        Agent::Delivery::PermanentWebhookResponse => error
      # Fail fast without consuming an attempt: revert the claim's
      # increment so the ledger keeps reporting 0 attempts for rows
      # that never became retryable.
      event.update!(webhook_status: "failed", webhook_attempts: attempt, webhook_last_error: short_error(error))
    rescue Agent::Delivery::RetryableWebhookResponse => error
      if attempts >= MAX_ATTEMPTS
        event.update!(webhook_status: "failed", webhook_last_error: short_error(error))
      else
        delay = error.retry_after || default_backoff_for(attempts)
        event.update!(webhook_status: "pending", webhook_last_error: short_error(error),
          webhook_next_attempt_at: Time.current + delay)
        self.class.set(wait: delay).perform_later(event.id, attempts)
      end
    rescue StandardError => error
      if attempts >= MAX_ATTEMPTS
        event.update!(webhook_status: "failed", webhook_last_error: short_error(error))
      else
        delay = default_backoff_for(attempts)
        event.update!(webhook_status: "pending", webhook_last_error: short_error(error),
          webhook_next_attempt_at: Time.current + delay)
        self.class.set(wait: delay).perform_later(event.id, attempts)
      end
    else
      event.update!(webhook_status: "delivered", webhook_last_error: nil)
    end
  end

  private
    # Default retry delay in seconds for attempt N (1-based), matching
    # the retry_on :polynomially_longer shape (executions**4 + 2) so a
    # scheduled retry and its recorded next_attempt_at agree.
    def default_backoff_for(attempts)
      (attempts**4) + 2
    end

    def short_error(error)
      "#{error.class}: #{error.message}".truncate(500)
    end
end
