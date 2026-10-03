module Message::Attachment
  extend ActiveSupport::Concern

  THUMBNAIL_MAX_WIDTH = 1200
  THUMBNAIL_MAX_HEIGHT = 800

  included do
    has_one_attached :attachment do |attachable|
      attachable.variant :thumb, resize_to_limit: [ THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT ]
    end
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
      ActiveRecord.after_all_transactions_commit do
        Rails.error.handle(context: { message_id: id }) do
          Message::AttachmentProcessingJob.perform_later(self)
        end
      end
    else
      # A processing error must not turn an already committed post into a
      # failed request. Transactional callers process in the retryable job.
      Rails.error.handle(context: { message_id: id }) { process_attachment_now }
    end
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
