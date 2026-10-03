module Message::Attachment
  extend ActiveSupport::Concern

  THUMBNAIL_MAX_WIDTH = 1200
  THUMBNAIL_MAX_HEIGHT = 800
  ATTACHMENT_PROCESSING_LEASE = 15.minutes

  included do
    has_one_attached :attachment do |attachable|
      attachable.variant :thumb, resize_to_limit: [ THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT ]
    end

    # Also covers attachment.attach on persisted messages and bot PATCHes.
    # Register after Active Storage's save callback; uploads still await commit.
    after_save :process_attachment, if: -> { !previously_new_record? && attachment_changes.key?("attachment") && attachment? }
  end

  module ClassMethods
    def create_with_attachment!(attributes)
      create!(attributes).tap(&:process_attachment)
    end
  end

  def attachment?
    attachment.attached?
  end

  def process_attachment
    return unless attachment?

    if ActiveRecord::Base.current_transaction.open?
      # Both the original multipart upload and a generated preview image
      # upload in Active Storage's record after_commit callbacks. Enqueue
      # only after those callbacks and the outermost transaction finish.
      process_attachment_later
    else
      # A processing error must not turn an already committed post into a
      # failed request. Transactional callers process in the retryable job.
      Rails.error.handle(context: { message_id: id }) do
        blob_id = attachment.blob_id
        token = SecureRandom.uuid
        if claim_attachment_processing(attachment.blob, token)
          begin
            process_attachment_now
          ensure
            release_attachment_processing(blob_id, token)
          end
        end
      end
    end
  end

  def process_attachment_later
    return unless attachment?

    blob_id = attachment.blob_id
    ActiveRecord.after_all_transactions_commit do
      Rails.error.handle(context: { message_id: id, blob_id: blob_id }) do
        blob = ActiveStorage::Blob.find_by(id: blob_id)
        if blob && attachment.blob_id == blob_id
          token = SecureRandom.uuid
          if claim_attachment_processing(blob, token)
            begin
              job = Message::AttachmentProcessingJob.perform_later(self, blob_id, token)
            ensure
              release_attachment_processing(blob_id, token) unless job && job.successfully_enqueued?
            end
          end
        end
      end
    end
  end

  def recover_attachment_preview
    if attachment? && attachment.video? && !attachment.blob.preview_image.attached?
      process_attachment_later
    end
  end

  # Claim only the scheduling state with an atomic update. Never hold a
  # database lock during decoding: JPEG uploads need their own real commit.
  # The lease also permits recovery after a lost job or worker crash.
  def claim_attachment_processing(blob, token)
    ActiveStorage::Blob.where(id: blob.id)
      .where("message_processing_token = ? OR message_processing_expires_at IS NULL OR message_processing_expires_at <= ?", token, Time.current)
      .update_all(message_processing_token: token, message_processing_expires_at: ATTACHMENT_PROCESSING_LEASE.from_now) == 1
  end

  def release_attachment_processing(blob_id, token)
    ActiveStorage::Blob.where(id: blob_id, message_processing_token: token)
      .update_all(message_processing_token: nil, message_processing_expires_at: nil)
  end

  def process_attachment_now
    ensure_attachment_analyzed
    process_attachment_thumbnail
  end

  private
    def ensure_attachment_analyzed
      attachment&.analyze
    end

    def process_attachment_thumbnail
      case
      when attachment.video?
        attachment.preview(format: :webp).processed
      when attachment.representable?
        attachment.representation(:thumb).processed
      end
    end
end
