class Agents::PinsController < ApplicationController
  include AgentAuthorization
  include AgentApiThrottle

  allow_agent_access only: %i[ create destroy ]

  # Bearer-only endpoint. Forgery protection stays on (see
  # Agents::MessagesController): a session-cookie request that trips it
  # gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  before_action :set_message, only: %i[ create destroy ]
  before_action :ensure_agent_token, only: %i[ create destroy ]
  require_agent_capability :post_messages, only: %i[ create destroy ]
  throttle_agent_api limit: 60, only: %i[ create destroy ]

  # POST /agents/messages/:id/pin (Bearer-only, JSON). Pins the message
  # in its room, posting the pin note as the agent. Idempotent: pinning
  # an already-pinned message succeeds without duplicating.
  def create
    pin = MessagePin.pin!(message: @message, pinner: Current.user)
    render json: { pinned: true, message_id: pin.message_id, pin_count: pin.room.message_pins.count }, status: :created
  rescue ActiveRecord::RecordInvalid
    render json: { pinned: true, message_id: @message.id, pin_count: @message.room.message_pins.count }, status: :ok
  rescue MessagePin::CapReachedError => error
    render json: { error: error.message }, status: :unprocessable_entity
  end

  # DELETE /agents/messages/:id/pin (Bearer-only, JSON). Unpinning a
  # message that is not pinned still succeeds.
  def destroy
    if (pin = @message.message_pins.first)
      pin.unpin!
    end

    render json: { pinned: false, message_id: @message.id, pin_count: @message.room.message_pins.count }
  end

  private
    def set_message
      @message = Current.user.reachable_messages.find(params[:id])
    rescue ActiveRecord::RecordNotFound
      head :not_found
    end

    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token?
    end

    def reject_session_request
      render json: { error: "Forbidden: Bearer agent token required" }, status: :forbidden
    end
end
