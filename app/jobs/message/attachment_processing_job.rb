class Message::AttachmentProcessingJob < ApplicationJob
  retry_on StandardError, wait: :polynomially_longer, attempts: 3 do |job, error|
    message, blob_id, token = job.arguments
    message.release_attachment_processing(blob_id || message.attachment.blob_id, token || job.job_id)
    raise error
  end
  discard_on ActiveJob::DeserializationError

  # The optional identity supports jobs queued before this fix was deployed.
  def perform(message, blob_id = message.attachment.blob_id, token = job_id)
    completed = false
    return unless message.attachment? && message.attachment.blob_id == blob_id
    return unless message.claim_attachment_processing(message.attachment.blob, token)

    message.process_attachment_now
    completed = true
    # Decoding can outlive an edit or deletion. Refresh every association,
    # and never touch or render the attachment the job used to know about.
    message.reload
    return unless message.attachment? && message.attachment.blob_id == blob_id
    # Analysis touches the message through its blob, but generating a
    # preview on an already analyzed video may not. Expire its cached
    # presentation in either case, then refresh the room or thread stream.
    # A signed upload may be attached to more than one message. Processing
    # is claimed per blob, so refresh every owner still using that blob.
    Message.joins(:attachment_attachment).where(active_storage_attachments: { blob_id: blob_id }).find_each do |owner|
      owner.reload
      next unless owner.attachment? && owner.attachment.blob_id == blob_id

      owner.touch
      owner.broadcast_replace_to owner.message_stream_target, :messages,
        target: [ owner, :presentation ], partial: "messages/presentation",
        attributes: { maintain_scroll: true }
    end
  rescue ActiveRecord::RecordNotFound
    completed = true
  ensure
    message.release_attachment_processing(blob_id, token) if completed || message.attachment.blob_id != blob_id
  end
end
