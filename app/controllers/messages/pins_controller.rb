class Messages::PinsController < ApplicationController
  before_action :set_message

  def create
    pin = MessagePin.pin!(message: @message, pinner: Current.user)
    respond_to do |format|
      format.html { redirect_back fallback_location: room_path(@message.room), notice: "Message pinned" }
      format.turbo_stream { head :ok }
      format.json { render json: pin_payload(pin), status: :created }
    end
  rescue ActiveRecord::RecordInvalid
    # Already pinned: pinning is idempotent.
    respond_to do |format|
      format.html { redirect_back fallback_location: room_path(@message.room), notice: "Message pinned" }
      format.turbo_stream { head :ok }
      format.json { render json: pin_payload(@message.message_pins.first), status: :ok }
    end
  rescue MessagePin::CapReachedError => error
    respond_to do |format|
      format.html { redirect_back fallback_location: room_path(@message.room), alert: error.message }
      format.turbo_stream { head :unprocessable_entity }
      format.json { render json: { error: error.message }, status: :unprocessable_entity }
    end
  end

  def destroy
    if (pin = @message.message_pins.first)
      pin.unpin!
    end

    respond_to do |format|
      format.html { redirect_back fallback_location: room_path(@message.room), notice: "Message unpinned" }
      format.turbo_stream { redirect_to room_pins_url(@message.room), status: :see_other }
      format.json { render json: { pinned: false, pin_count: @message.room.message_pins.count } }
    end
  end

  private
    def set_message
      @message = Current.user.reachable_messages.find(params[:message_id])
    end

    def pin_payload(pin)
      { pinned: true, pin_count: pin.room.message_pins.count }
    end
end
