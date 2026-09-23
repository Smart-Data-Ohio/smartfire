class ScheduledMessagesController < ApplicationController
  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_active_human
  before_action :no_store_response!

  # GET /scheduled_messages. The author's upcoming rows, rows stranded
  # by lost access (shown so they can be cancelled; the dispatcher
  # drops them with an inbox item when they come due), plus sent and
  # dropped history.
  def index
    rows = Current.user.scheduled_messages.includes(:room, :thread).ordered.to_a
    @upcoming = rows.select(&:pending?).select(&:sendable?)
    @stranded = rows.select(&:pending?).reject(&:sendable?)
    @past = rows.reject(&:pending?).sort_by { |row| [ row.send_at, row.id ] }.reverse
  end

  # POST /rooms/:room_id/scheduled_messages. Schedules the composer's
  # draft for send_at (UTC ISO8601 from the client, resolved in the
  # author's time zone). An optional thread_id schedules a thread reply.
  def create
    room = Current.user.memberships.joins(:room).merge(Room.alive).find_by!(room_id: params[:room_id]).room
    permitted = params.require(:scheduled_message).permit(:markdown_source, :send_at, :thread_id, :reply_to_message_id)
    thread = room.channel_threads.find(permitted[:thread_id]) if permitted[:thread_id].present?

    scheduled = Current.user.scheduled_messages.build(
      room: room, thread: thread,
      markdown_source: permitted.require(:markdown_source),
      reply_to_message_id: create_reply_id(room, thread, permitted[:reply_to_message_id]),
      send_at: parse_send_at!(permitted[:send_at])
    )

    if scheduled.save
      respond_to do |format|
        format.html { redirect_to scheduled_messages_path, notice: "Message scheduled for #{scheduled.send_at.in_time_zone(Current.user.time_zone_or_default).to_fs(:long)}." }
        format.json { render json: scheduled_payload(scheduled), status: :created }
      end
    else
      respond_to do |format|
        format.html { redirect_back fallback_location: scheduled_messages_path, alert: scheduled.errors.full_messages.to_sentence }
        format.json { render json: { errors: scheduled.errors.to_hash }, status: :unprocessable_entity }
      end
    end
  rescue ArgumentError => error
    respond_to do |format|
      format.html { redirect_back fallback_location: scheduled_messages_path, alert: error.message }
      format.json { render json: { error: error.message }, status: :unprocessable_entity }
    end
  end

  # PATCH /scheduled_messages/:id. Edits a pending row's text or time.
  def update
    scheduled = Current.user.scheduled_messages.pending.find(params[:id])
    return if refuse_when_claimed(scheduled)

    scheduled.assign_attributes(update_attributes)

    if scheduled.save
      respond_to do |format|
        format.html { redirect_to scheduled_messages_path, notice: "Scheduled message updated." }
        format.json { render json: scheduled_payload(scheduled) }
      end
    else
      respond_to do |format|
        format.html { redirect_to scheduled_messages_path, alert: scheduled.errors.full_messages.to_sentence }
        format.json { render json: { errors: scheduled.errors.to_hash }, status: :unprocessable_entity }
      end
    end
  rescue ArgumentError => error
    respond_to do |format|
      format.html { redirect_to scheduled_messages_path, alert: error.message }
      format.json { render json: { error: error.message }, status: :unprocessable_entity }
    end
  end

  # DELETE /scheduled_messages/:id. Cancels a pending row.
  def destroy
    scheduled = Current.user.scheduled_messages.pending.find(params[:id])
    return if refuse_when_claimed(scheduled)

    scheduled.destroy!

    respond_to do |format|
      format.html { redirect_to scheduled_messages_path, notice: "Scheduled message cancelled." }
      format.json { head :no_content }
    end
  end

  # POST /scheduled_messages/:id/send_now. Posts a pending row
  # immediately, re-checking access like the dispatcher does.
  def send_now
    scheduled = Current.user.scheduled_messages.pending.find(params[:id])

    if ScheduledMessage::Dispatcher.dispatch_now!(scheduled)
      respond_to do |format|
        format.html { redirect_to scheduled_messages_path, notice: "Message sent." }
        format.json { render json: scheduled_payload(scheduled.reload) }
      end
    elsif scheduled.reload.dropped?
      alert = drop_alert_for(scheduled)
      respond_to do |format|
        format.html { redirect_to scheduled_messages_path, alert: alert }
        format.json { render json: { error: alert }, status: :unprocessable_entity }
      end
    else
      # Still pending: the periodic runner holds its claim and will send
      # it momentarily.
      respond_to do |format|
        format.html { redirect_to scheduled_messages_path, notice: "Message is sending." }
        format.json { render json: scheduled_payload(scheduled), status: :accepted }
      end
    end
  end

  private
    def ensure_active_human
      head :forbidden unless Current.user&.active? && !Current.user.bot?
    end

    def create_reply_id(room, thread, raw)
      return if raw.blank?

      scope = thread ? thread.messages : room.root_messages
      scope.find(raw).id
    rescue ActiveRecord::RecordNotFound
      raise ArgumentError, "Reply target is not in this conversation"
    end

    def update_attributes
      permitted = params.require(:scheduled_message).permit(:markdown_source, :send_at)
      permitted[:send_at] = parse_send_at!(permitted[:send_at]) if permitted.key?(:send_at)
      permitted.to_h.symbolize_keys
    end

    def parse_send_at!(value = params.dig(:scheduled_message, :send_at))
      parsed = Time.zone.parse(value.to_s)
      raise ArgumentError, "Send time is invalid" if parsed.nil?

      parsed
    end

    # The dispatcher is about to post a claimed row: an edit would be
    # silently lost and a cancel would race the send, so refuse with a
    # clear notice instead. Renders and returns true when refused.
    def refuse_when_claimed(scheduled)
      return false unless scheduled.claimed?

      alert = "That message is sending right now; try again in a moment."
      respond_to do |format|
        format.html { redirect_to scheduled_messages_path, alert: alert }
        format.json { render json: { error: alert }, status: :conflict }
      end
      true
    end

    def drop_alert_for(scheduled)
      if scheduled.drop_reason.present?
        "The scheduled message was not sent (#{scheduled.drop_reason})."
      else
        "You no longer have access to that room, so the message was not sent."
      end
    end

    def scheduled_payload(scheduled)
      {
        id: scheduled.id, room_id: scheduled.room_id, thread_id: scheduled.thread_id,
        markdown_source: scheduled.markdown_source, send_at: scheduled.send_at,
        sent_at: scheduled.sent_at, dropped_at: scheduled.dropped_at
      }
    end
end
