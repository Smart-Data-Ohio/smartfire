class ApplicationJob < ActiveJob::Base
  # Errors worth another attempt. ActiveRecord translates a SQLite lock
  # timeout (SQLite3::BusyException) into StatementTimeout, so both the raw
  # error and the translation are listed; Deadlocked covers other adapters.
  # Retries wait with polynomial backoff through resque-scheduler's delayed
  # queue, which the periodic runner drains on every tick.
  TRANSIENT_ERRORS = [
    Timeout::Error, Net::OpenTimeout, Net::ReadTimeout, Net::WriteTimeout,
    ActiveRecord::Deadlocked, ActiveRecord::StatementTimeout, SQLite3::BusyException
  ].freeze

  retry_on(*TRANSIENT_ERRORS, wait: :polynomially_longer)

  # Most jobs are safe to ignore if the underlying records are no longer available
  discard_on ActiveJob::DeserializationError
end
