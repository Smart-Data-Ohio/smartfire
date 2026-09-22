class Calendar::DisconnectCleanupJob < ApplicationJob
  # Post-disconnect cleanup: the account row and local entries are already
  # gone, so the disconnect request returned without waiting on Google.
  # This job removes the remote copies with the token snapshot captured
  # at disconnect time, then revokes the grant. Deletes run before the
  # revoke: revoking first would unauthenticate the deletes. Deactivation
  # reuses this job with an empty id list for a revoke-only run. Best
  # effort throughout, like disconnect always was: failures are logged
  # (never with tokens) and the job does not retry.
  self.enqueue_after_transaction_commit = true

  # Credentials travel as an encrypted blob, and stay out of the logs
  # even so: argument logging would otherwise record them on enqueue.
  self.log_arguments = false

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

  def perform(google_event_ids, credentials_blob)
    credentials = if credentials_blob.is_a?(Hash)
      credentials_blob # Legacy raw snapshot from before encryption.
    else
      self.class.decrypt_credentials(credentials_blob)
    end
    return if credentials.blank?

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
      rescue Google::Client::Error => error
        Rails.logger.warn "Calendar::DisconnectCleanupJob could not remove #{google_event_id}: #{error.class}"
      end
    end

    begin
      Google::Client.revoke_token(credentials[:refresh_token])
    rescue Google::Client::Error => error
      Rails.logger.warn "Calendar::DisconnectCleanupJob could not revoke the grant: #{error.class}"
    end
  end
end
