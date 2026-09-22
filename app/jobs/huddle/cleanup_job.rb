class Huddle::CleanupJob < ApplicationJob
  # Cleanups carry their own lease and backoff schedule (next_attempt_at);
  # an ActiveJob retry would bypass it. attempts: 1 opts out of the
  # inherited transient retries.
  retry_on(*ApplicationJob::TRANSIENT_ERRORS, attempts: 1)

  def perform(cleanup_id)
    HuddleCleanup.find_by(id: cleanup_id)&.perform_from_queue!
  end
end
