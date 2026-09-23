class Calendar::InboundSyncJob < ApplicationJob
  # Idempotent: remote state is re-read at run time and attendance is set
  # to the converged value. Transient Google failures retry with
  # backoff; permanent ones are recorded on the channel.
  retry_on Google::Client::Unavailable, wait: :polynomially_longer, attempts: 8

  def perform(user_id)
    Calendar::InboundSync.sync(user_id)
  end
end
