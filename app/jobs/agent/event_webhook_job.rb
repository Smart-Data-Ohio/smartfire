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
  def perform(event_id)
    event = AgentEvent.find_by(id: event_id)
    return unless event
    return if event.webhook_status == "delivered"

    webhook = event.agent.user.webhook
    if webhook.nil?
      event.update!(webhook_status: "none")
      return
    end

    begin
      Agent::Delivery.post_event_webhook!(webhook, event, agent: event.agent)
    rescue Agent::Delivery::UndeliverableWebhook, RestrictedHTTP::Violation, Surfguard::Unresolvable, URI::InvalidURIError,
        Agent::Delivery::PermanentWebhookResponse => error
      event.update!(webhook_status: "failed", webhook_last_error: short_error(error))
    rescue Agent::Delivery::RetryableWebhookResponse => error
      attempts = event.webhook_attempts.to_i + 1
      if attempts >= MAX_ATTEMPTS
        event.update!(webhook_status: "failed", webhook_attempts: attempts, webhook_last_error: short_error(error))
      else
        event.update!(webhook_status: "pending", webhook_attempts: attempts, webhook_last_error: short_error(error))
        if error.retry_after
          retry_job(wait: error.retry_after)
        else
          raise
        end
      end
    rescue StandardError => error
      attempts = event.webhook_attempts.to_i + 1
      if attempts >= MAX_ATTEMPTS
        event.update!(webhook_status: "failed", webhook_attempts: attempts, webhook_last_error: short_error(error))
      else
        event.update!(webhook_status: "pending", webhook_attempts: attempts, webhook_last_error: short_error(error))
        raise
      end
    else
      event.update!(webhook_status: "delivered", webhook_attempts: event.webhook_attempts.to_i + 1, webhook_last_error: nil)
    end
  end

  private
    def short_error(error)
      "#{error.class}: #{error.message}".truncate(500)
    end
end
