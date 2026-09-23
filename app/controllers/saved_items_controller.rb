class SavedItemsController < ApplicationController
  STATUSES = %w[ in_progress done ].freeze

  before_action :set_status_filter, only: :index
  before_action :no_store_response!

  def index
    @saved_items = accessible_saved_items
      .includes(message: [ :room, :rich_text_body, { creator: :avatar_attachment } ])
      .ordered
    @saved_items = @saved_items.where(status: @status_filter) if @status_filter != "all"
    @counts = accessible_saved_items.group(:status).count
  end

  # Saving is idempotent per user per message: re-saving updates the
  # reminder instead of duplicating the item.
  def create
    message = Current.user.reachable_messages.find(params[:message_id])

    saved_item = Current.user.saved_items.find_or_initialize_by(message:)
    saved_item.remind_at = parsed_remind_at!
    saved_item.save!

    respond_to do |format|
      format.html { redirect_back fallback_location: saved_items_path, notice: "Saved for later" }
      format.turbo_stream { head :ok }
      format.json { render json: saved_item_payload(saved_item), status: :created }
    end
  rescue ArgumentError
    render_invalid_reminder
  rescue ActiveRecord::RecordInvalid => error
    respond_to do |format|
      format.html { redirect_back fallback_location: saved_items_path, alert: error.record.errors.full_messages.to_sentence }
      format.turbo_stream { head :unprocessable_entity }
      format.json { render json: { errors: error.record.errors.to_hash }, status: :unprocessable_entity }
    end
  end

  def update
    saved_item = accessible_saved_items.find(params[:id])
    status = params.require(:saved_item).require(:status)

    unless STATUSES.include?(status.to_s)
      return render_invalid_status
    end

    saved_item.update!(status:)

    respond_to do |format|
      format.html { redirect_to saved_items_path(status: redirect_filter) }
      format.turbo_stream { redirect_to saved_items_path(status: redirect_filter), status: :see_other }
      format.json { render json: saved_item_payload(saved_item) }
    end
  end

  def destroy
    accessible_saved_items.find(params[:id]).destroy!

    respond_to do |format|
      format.html { redirect_to saved_items_path(status: redirect_filter) }
      format.turbo_stream { redirect_to saved_items_path(status: redirect_filter), status: :see_other }
      format.json { head :no_content }
    end
  end

  private
    def set_status_filter
      @status_filter = STATUSES.include?(params[:status].to_s) ? params[:status].to_s : "all"
    end

    def accessible_saved_items
      SavedItem.accessible_to(Current.user)
    end

    # The client computes the reminder moment in the viewer's own time
    # zone (for "tomorrow 9am") and submits it as UTC ISO8601. Past
    # values fail the model validation; unparseable values raise.
    def parsed_remind_at!
      raw = params.dig(:saved_item, :remind_at).presence || params[:remind_at].presence
      return if raw.blank?

      Time.zone.parse(raw.to_s) || raise(ArgumentError, "unparseable remind_at")
    end

    def render_invalid_reminder
      respond_to do |format|
        format.html { redirect_back fallback_location: saved_items_path, alert: "Reminder time is invalid" }
        format.turbo_stream { head :unprocessable_entity }
        format.json { render json: { errors: { remind_at: [ "is invalid" ] } }, status: :unprocessable_entity }
      end
    end

    def redirect_filter
      STATUSES.include?(params[:status].to_s) ? params[:status].to_s : "all"
    end

    def render_invalid_status
      respond_to do |format|
        format.html { redirect_to saved_items_path(status: redirect_filter), alert: "Status must be in progress or done" }
        format.turbo_stream { redirect_to saved_items_path(status: redirect_filter), status: :see_other, alert: "Status must be in progress or done" }
        format.json { render json: { error: "Status must be in progress or done" }, status: :unprocessable_content }
      end
    end

    def saved_item_payload(saved_item)
      {
        id: saved_item.id,
        message_id: saved_item.message_id,
        status: saved_item.status,
        remind_at: saved_item.remind_at&.utc,
        reminded_at: saved_item.reminded_at&.utc
      }
    end

    def no_store_response!
      response.headers["Cache-Control"] = "no-store"
      response.headers["Pragma"] = "no-cache"
    end
end
