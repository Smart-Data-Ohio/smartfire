class Agents::PollsController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: %i[ create show ]

  # Bearer-only endpoint. Forgery protection stays on (see
  # Agents::MessagesController): a session-cookie request that trips it
  # gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  before_action :ensure_agent_token, only: %i[ create show ]
  throttle_agent_api limit: 60, only: :create
  throttle_agent_api limit: 120, only: :show

  # POST /rooms/:room_id/agents/polls (Bearer-only, JSON). Creates a
  # root message carrying a poll (question, 2-10 options, multiple,
  # anonymous, closes_at). The flow lives in Agents::Polls, shared with
  # the MCP create_poll tool.
  def create
    render_poll_result Agents::Polls.create(
      agent: Current.agent, room_id: params[:room_id],
      question: params[:question], options: params[:options],
      multiple: params[:multiple], anonymous: params[:anonymous],
      closes_at: params[:closes_at]
    )
  end

  # GET /rooms/:room_id/agents/polls/:id (Bearer-only, JSON). Reads the
  # poll with live counts. Shared with the MCP get_poll tool.
  def show
    render_poll_result Agents::Polls.show(
      agent: Current.agent, room_id: params[:room_id], poll_id: params[:id]
    )
  end

  private
    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token?
    end

    def reject_session_request
      render json: { error: "Forbidden: #{"Bearer"} agent token required" }, status: :forbidden
    end

    def render_poll_result(result)
      if result.ok?
        render json: result.payload, status: result.status
      elsif result.status == :not_found
        head :not_found
      else
        render json: result.failure_body, status: result.status
      end
    end
end
