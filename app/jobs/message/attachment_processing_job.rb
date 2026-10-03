class Message::AttachmentProcessingJob < ApplicationJob
  retry_on StandardError, wait: :polynomially_longer, attempts: 3 do |job, error|
    message, blob_id, token = job.arguments
    message.fail_attachment_processing(blob_id || message.attachment.blob_id, token || job.job_id)
    raise error
  end
  discard_on ActiveJob::DeserializationError

  # The optional identity supports jobs queued before this fix was deployed.
  def perform(message, blob_id = message.attachment.blob_id, token = job_id)
    completed = false
    owners = Message.joins(:attachment_attachment).where(active_storage_attachments: { blob_id: blob_id })
    processor = owners.find_by(id: message.id) || owners.first
    unless processor
      completed = true
      return
    end
    return unless message.claim_attachment_processing(processor.attachment.blob, token)

    processor.process_attachment_now
    completed = true
    # Analysis touches the message through its blob, but generating a
    # preview on an already analyzed video may not. Expire its cached
    # presentation in either case, then refresh the room or thread stream.
    # A signed upload may be attached to more than one message. Processing
    # is claimed per blob, so refresh every owner still using that blob.
    owners.find_each do |owner|
      owner.reload
      next unless owner.attachment? && owner.attachment.blob_id == blob_id

      owner.touch
      owner.broadcast_replace_to owner.message_stream_target, :messages,
        target: [ owner, :presentation ], partial: "messages/presentation",
        attributes: { maintain_scroll: true }
    rescue ActiveRecord::RecordNotFound
      # One owner can disappear while decoding or rendering; continue with
      # the others rather than suppressing their completion broadcasts.
      next
    end
  rescue ActiveRecord::RecordNotFound
    completed = true
  ensure
    message.release_attachment_processing(blob_id, token) if completed
  end
end
