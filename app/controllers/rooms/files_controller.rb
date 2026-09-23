# A room's file browser: uploaded attachments plus Drive files pinned
# through the picker, each newest-first. Drive rows carry picker-only
# data (the stored file id); names, kinds, and times resolve per viewer
# in the browser through the drive-link controller, so type filters and
# filename search apply to uploads only.
class Rooms::FilesController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  FILES_PER_PAGE = 30
  MAX_PAGE = 20
  TYPES = %w[all images videos documents other].freeze
  DOCUMENT_PATTERNS = [
    "application/pdf", "text/%",
    "application/msword", "application/vnd.ms-%",
    "application/vnd.openxmlformats-officedocument.%",
    "application/vnd.oasis.opendocument.%"
  ].freeze

  def index
    @type = params[:type].to_s.presence_in(TYPES) || "all"
    @filename_query = params[:filename].to_s.strip
    @upload_page = upload_page_number
    @drive_page = drive_page_number

    uploads_plus_probe = upload_scope.to_a
    @has_more_uploads = uploads_plus_probe.size > upload_page_size
    @uploads = uploads_plus_probe.first(upload_page_size)

    drive_plus_probe = drive_scope.to_a
    @has_more_drive = drive_plus_probe.size > drive_page_size
    @drive_files = drive_plus_probe.first(drive_page_size)
  end

  private
    def upload_scope
      scope = ActiveStorage::Attachment
        .where(record_type: "Message", name: "attachment", record_id: @room.messages.select(:id))
        .joins(:blob).preload(:blob, record: [ :creator, :room ])
        .order("active_storage_attachments.created_at DESC, active_storage_attachments.id DESC")

      scope = filter_by_type(scope)
      if @filename_query.present?
        scope = scope.where("LOWER(active_storage_blobs.filename) LIKE ? ESCAPE '\\'",
          "%#{ActiveRecord::Base.sanitize_sql_like(@filename_query.downcase)}%")
      end

      scope.limit(upload_page_number * FILES_PER_PAGE + 1)
    end

    def drive_scope
      DriveAttachment.where(message_id: @room.messages.select(:id))
        .includes(message: [ :creator, :room ])
        .order(created_at: :desc, id: :desc)
        .limit(drive_page_number * FILES_PER_PAGE + 1)
    end

    def filter_by_type(scope)
      case @type
      when "images"
        scope.where("active_storage_blobs.content_type LIKE ?", "image/%")
      when "videos"
        scope.where("active_storage_blobs.content_type LIKE ?", "video/%")
      when "documents"
        conditions = DOCUMENT_PATTERNS.map { "active_storage_blobs.content_type LIKE ?" }.join(" OR ")
        scope.where(conditions, *DOCUMENT_PATTERNS)
      when "other"
        conditions = DOCUMENT_PATTERNS.map { "active_storage_blobs.content_type LIKE ?" }.join(" OR ")
        scope.where(
          "active_storage_blobs.content_type NOT LIKE ? AND active_storage_blobs.content_type NOT LIKE ? AND NOT (#{conditions})",
          "image/%", "video/%", *DOCUMENT_PATTERNS
        )
      else
        scope
      end
    end

    def upload_page_number
      [ [ params[:page].to_s.to_i, 1 ].max, MAX_PAGE ].min
    end

    def drive_page_number
      [ [ params[:drive_page].to_s.to_i, 1 ].max, MAX_PAGE ].min
    end

    def upload_page_size
      upload_page_number * FILES_PER_PAGE
    end

    def drive_page_size
      drive_page_number * FILES_PER_PAGE
    end
end
