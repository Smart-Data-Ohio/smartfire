class Agents::ContextsController < ApplicationController
  include AgentAuthorization
  include AgentApiThrottle

  allow_agent_access only: :show

  before_action :ensure_agent_token, only: :show
  require_agent_capability :read_messages, only: :show
  throttle_agent_api limit: 120, only: :show

  # GET /agents/context?message_id=&thread_id=&limit= (Bearer-only, JSON).
  # Returns the triggering message, its thread summary and root message,
  # the last N messages of the same conversation ending at the trigger
  # (default 30, max 100) with agent/human authors, and the room. One of
  # message_id or thread_id is required. The lookup lives in
  # Agents::ContextBuilder, shared with the MCP get_context tool.
  def show
    no_store_response!

    result = Agents::ContextBuilder.build(
      agent: Current.agent,
      message_id: params[:message_id],
      thread_id: params[:thread_id],
      limit: params[:limit],
      presenter: self
    )

    if result.ok?
      render json: result.payload
    elsif result.status == :not_found
      head :not_found
    else
      render json: result.failure_body, status: result.status
    end
  end

  private
    def ensure_agent_token
      unless authenticated_by.agent_token? && Current.agent
        render json: { error: "Forbidden: #{"Bearer"} agent token required" }, status: :forbidden
      end
    end
end
