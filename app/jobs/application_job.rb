class ApplicationJob < ActiveJob::Base
  # Errors worth another attempt. ActiveRecord translates a SQLite lock
  # timeout (SQLite3::BusyException) into StatementTimeout, so both the raw
  # error and the translation are listed; Deadlocked covers other adapters.
  TRANSIENT_ERRORS = [
    Timeout::Error, Net::OpenTimeout, Net::ReadTimeout, Net::WriteTimeout,
    ActiveRecord::Deadlocked, ActiveRecord::StatementTimeout, SQLite3::BusyException
  ].freeze

  retry_on(*TRANSIENT_ERRORS, wait: :polynomially_longer)

  # Most jobs are safe to ignore if the underlying records are no longer available
  discard_on ActiveJob::DeserializationError

  private
    # Production runs plain resque with no scheduler, so a retry with a wait
    # would raise NotImplementedError from the adapter instead of retrying.
    # Re-enqueue immediately there (the queue itself spaces the attempts);
    # adapters with scheduling keep the polynomial backoff above.
    def retry_job(options = {})
      if options[:wait] && !scheduled_retries_supported?
        options = options.except(:wait)
      end

      super(options)
    end

    def scheduled_retries_supported?
      !queue_adapter.is_a?(ActiveJob::QueueAdapters::ResqueAdapter) ||
        Resque.respond_to?(:enqueue_at_with_queue)
    end
end
