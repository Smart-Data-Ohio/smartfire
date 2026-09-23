class Agents::PinsController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: %i[ create destroy ]

  # Bearer-only endpoint. Forgery protection stays on (see
  # Agents::MessagesController): a session-cookie request that trips it
  # gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  before_action :ensure_agent_token, only: %i[ create destroy ]
  throttle_agent_api limit: 60, only: %i[ create destroy ]

  # POST /agents/messages/:id/pin (Bearer-only, JSON). Pins the message
  # in its room, posting the pin note as the agent. Idempotent: pinning
  # an already-pinned message succeeds without duplicating. The flow
  # lives in Agents::Pins, shared with the MCP pin_message tool.
  def create
    render_pin_result Agents::Pins.pin(agent: Current.agent, message_id: params[:id])
  end

  # DELETE /agents/messages/:id/pin (Bearer-only, JSON). Unpinning a
  # message that is not pinned still succeeds. Shared with the MCP
  # unpin_message tool through Agents::Pins.
  def destroy
    render_pin_result Agents::Pins.unpin(agent: Current.agent, message_id: params[:id])
  end

  private
    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token?
    end

    def reject_session_request
      render json: { error: "Forbidden: #{"Bearer"} agent token required" }, status: :forbidden
    end

    def render_pin_result(result)
      if result.ok?
        render json: result.payload, status: result.status
      elsif result.status == :not_found
        head :not_found
      else
        render json: result.failure_body, status: result.status
      end
    end
end
