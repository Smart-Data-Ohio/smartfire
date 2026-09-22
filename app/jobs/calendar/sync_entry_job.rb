class Calendar::SyncEntryJob < ApplicationJob
  # Event updates and cancellations enqueue inside transactions; deferring
  # the enqueue to after commit keeps the worker from running on
  # pre-commit state.
  self.enqueue_after_transaction_commit = true

  # Idempotent: the desired state is recomputed from the database at run
  # time. Transient Google failures (rate limits, timeouts, connection
  # drops) are re-raised by the reconciler and retried here with
  # backoff; permanent failures are recorded on the entry for the next
  # change to retry.
  retry_on Google::Client::Unavailable, wait: :polynomially_longer, attempts: 8

  def perform(event_id, user_id)
    Calendar::EntrySync.sync(event_id, user_id)
  end
end
