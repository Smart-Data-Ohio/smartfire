class Calendar::DisconnectCleanupJob < ApplicationJob
  # Post-disconnect cleanup: the account row and local entries are already
  # gone, so the disconnect request returned without waiting on Google.
  # This job removes the remote copies with the token snapshot captured
  # at disconnect time, then revokes the grant. Deletes run before the
  # revoke: revoking first would unauthenticate the deletes, and a
  # transient delete failure retries instead of revoking early.
  # Deactivation reuses this job with an empty id list for a revoke-only
  # run. Permanent failures are logged (never with tokens); transient
  # ones retry, and an exhausted retry logs at error level and reports
  # to the error service, since the account row the failure could be
  # recorded on is gone.
  self.enqueue_after_transaction_commit = true

  # Credentials travel as an encrypted blob, and stay out of the logs
  # even so: argument logging would otherwise record them on enqueue.
  self.log_arguments = false

  retry_on Google::Client::Unavailable, wait: :polynomially_longer, attempts: 8 do |job, error|
    Rails.logger.error "Calendar::DisconnectCleanupJob failed after retries: #{error.class}"
    Rails.error.report(error, context: { account_id: job.arguments.third })
  end

  CREDENTIALS_PURPOSE = "calendar/disconnect-cleanup"
  CREDENTIALS_EXPIRES_IN = 1.day

  def self.credentials_encryptor
    @credentials_encryptor ||= ActiveSupport::MessageEncryptor.new(
      Rails.application.key_generator.generate_key(CREDENTIALS_PURPOSE, ActiveSupport::MessageEncryptor.key_len)
    )
  end

  def self.encrypt_credentials(snapshot)
    credentials_encryptor.encrypt_and_sign(snapshot, expires_in: CREDENTIALS_EXPIRES_IN, purpose: CREDENTIALS_PURPOSE)
  end

  # Nil when the blob expired, was tampered with, or was encrypted under
  # a rotated key: there is nothing cleanup could authenticate with.
  def self.decrypt_credentials(blob)
    return nil unless blob.is_a?(String)

    credentials_encryptor.decrypt_and_verify(blob, purpose: CREDENTIALS_PURPOSE)
  rescue ActiveSupport::MessageEncryptor::InvalidMessage
    nil
  end

  def perform(google_event_ids, credentials_blob, account_id = nil)
    credentials = if credentials_blob.is_a?(Hash)
      credentials_blob # Legacy raw snapshot from before encryption.
    else
      self.class.decrypt_credentials(credentials_blob)
    end
    if credentials.blank?
      Rails.logger.warn "Calendar::DisconnectCleanupJob skipped cleanup for account #{account_id || "unknown"}: credentials expired or unreadable"
      return
    end

    credentials = credentials.with_indifferent_access
    client = Google::Client.new(
      Google::Client::SnapshotCredentials.new(
        access_token: credentials[:access_token],
        refresh_token: credentials[:refresh_token],
        access_token_expires_at: credentials[:access_token_expires_at]
      )
    )

    Array(google_event_ids).each do |google_event_id|
      begin
        client.delete_event(google_event_id)
      rescue Google::Client::NotFound, Google::Client::Unauthorized
        nil
      rescue Google::Client::Unavailable
        raise
      rescue Google::Client::Error => error
        Rails.logger.warn "Calendar::DisconnectCleanupJob could not remove #{google_event_id}: #{error.class}"
      end
    end

    # Revoking after permanent delete failures (or an already-dead
    # grant) is intended: the member asked to disconnect, so the grant
    # goes away even when a remote copy could not be removed. Only
    # transient failures retry instead of revoking.
    begin
      revoked = Google::Client.revoke_token(credentials[:refresh_token])
      Rails.logger.warn "Calendar::DisconnectCleanupJob could not revoke the grant: rejected" unless revoked
    rescue Google::Client::Unavailable
      raise
    rescue Google::Client::Error => error
      Rails.logger.warn "Calendar::DisconnectCleanupJob could not revoke the grant: #{error.class}"
    end
  end
end
