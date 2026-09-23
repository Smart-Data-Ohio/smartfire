class Calendar::MeetLinkJob < ApplicationJob
  self.enqueue_after_transaction_commit = true

  # Idempotent: provision! is a no-op once the link exists. Transient
  # Google failures retry with backoff; permanent ones are logged and the
  # next event edit retries.
  retry_on Google::Client::Unavailable, wait: :polynomially_longer, attempts: 8

  def perform(event_id)
    event = Event.find_by(id: event_id)
    Calendar::MeetLink.provision!(event) if event
  end
end
