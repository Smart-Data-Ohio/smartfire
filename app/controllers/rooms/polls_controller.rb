class Rooms::PollsController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_active_human
  before_action :set_poll, only: :vote

  # POST /rooms/:room_id/polls. Posts a root message carrying a poll:
  # the question is the message text, with 2-10 options, single or
  # multiple choice, an optional close time, and optional anonymity.
  def create
    permitted = params.require(:poll).permit(:question, :multiple, :anonymous, :closes_at, options: [])
    closes_at = parse_closes_at(permitted[:closes_at])

    ActiveRecord::Base.transaction do
      @message = @room.root_messages.create!(
        creator: Current.user, markdown_source: permitted.require(:question)
      )
      @poll = Poll.create_for_message!(
        message: @message, labels: permitted[:options],
        multiple: boolean_param(permitted[:multiple]), anonymous: boolean_param(permitted[:anonymous]),
        closes_at: closes_at
      )
    end

    @message.process_attachment
    @message.broadcast_create
    Message::BotWebhookFanout.deliver_for(@message)

    respond_to do |format|
      format.html { redirect_to room_path(@room), notice: "Poll posted." }
      format.json { render json: @poll.results_payload(viewer: Current.user), status: :created }
    end
  rescue ActiveRecord::RecordInvalid => error
    respond_to do |format|
      format.html { redirect_to room_path(@room), alert: error.record.errors.full_messages.to_sentence }
      format.json { render json: { errors: error.record.errors.to_hash }, status: :unprocessable_entity }
    end
  rescue ActionController::ParameterMissing => error
    respond_to do |format|
      format.html { redirect_to room_path(@room), alert: error.message }
      format.json { render json: { error: error.message }, status: :unprocessable_entity }
    end
  end

  # POST /rooms/:room_id/polls/:id/vote. Replaces the voter's ballot
  # with option_ids (empty retracts). The card broadcasts live to the
  # room; the sender's response replaces it directly too.
  def vote
    @poll.cast_vote!(Current.user, params[:option_ids])

    respond_to do |format|
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          helpers.dom_id(@poll, :card), partial: "polls/poll", locals: { poll: @poll }
        )
      end
      format.json { render json: @poll.results_payload(viewer: Current.user) }
      format.html { redirect_to room_path(@room) }
    end
  rescue ActiveRecord::RecordInvalid => error
    respond_to do |format|
      format.turbo_stream { render turbo_stream: turbo_stream.replace(
        helpers.dom_id(@poll, :card), partial: "polls/poll", locals: { poll: @poll, vote_error: error.record.errors.full_messages.to_sentence }
      ), status: :unprocessable_entity }
      format.json { render json: { errors: error.record.errors.to_hash }, status: :unprocessable_entity }
      format.html { redirect_to room_path(@room), alert: error.record.errors.full_messages.to_sentence }
    end
  end

  private
    def ensure_active_human
      head :forbidden unless Current.user&.active? && !Current.user.bot?
    end

    def set_poll
      @poll = Poll.joins(:message).where(messages: { room_id: @room.id }).find(params[:id])
    end

    def boolean_param(value)
      ActiveModel::Type::Boolean.new.cast(value)
    end

    # The builder posts a zone-less datetime-local, interpreted in the
    # member's own time zone (this request already renders in it).
    def parse_closes_at(raw)
      raw = raw.to_s.strip
      return if raw.blank?

      Time.zone.parse(raw)
    end
end
