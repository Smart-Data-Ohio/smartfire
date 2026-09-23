class Agents::SlashCommandsController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: %i[ create destroy ]

  # Bearer-only endpoint. Forgery protection stays on (see
  # Agents::MessagesController): a session-cookie request that trips it
  # gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  before_action :ensure_agent_token, only: %i[ create destroy ]
  throttle_agent_api limit: 60, only: %i[ create destroy ]

  # POST /rooms/:room_id/agents/slash_commands (Bearer-only, JSON).
  # Registers a custom slash command for the room, delivered to the
  # agent as a slash_command event when a member invokes it. The flow
  # lives in Agents::SlashCommands, shared with the MCP
  # register_slash_command tool.
  def create
    render_command_result Agents::SlashCommands.register(
      agent: Current.agent, room_id: params[:room_id],
      name: params[:name], description: params[:description]
    )
  end

  # DELETE /rooms/:room_id/agents/slash_commands/:name (Bearer-only,
  # JSON). Unregistering a name the agent does not own answers 404.
  # Shared with the MCP unregister_slash_command tool.
  def destroy
    render_command_result Agents::SlashCommands.unregister(
      agent: Current.agent, room_id: params[:room_id], name: params[:name]
    )
  end

  private
    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token?
    end

    def reject_session_request
      render json: { error: "Forbidden: #{"Bearer"} agent token required" }, status: :forbidden
    end

    def render_command_result(result)
      if result.ok?
        render json: result.payload, status: result.status
      elsif result.status == :not_found
        head :not_found
      else
        render json: result.failure_body, status: result.status
      end
    end
end
