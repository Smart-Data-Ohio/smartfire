class Agents::DmsController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: :create

  before_action :ensure_agent_token, only: :create
  throttle_agent_api limit: 60, only: :create

  # POST /agents/dms (Bearer-only, JSON). Opens (or creates) the 1:1 DM
  # between the agent's bot user and a human target, then posts the
  # agent's message. Takes user_id plus a message object (or top-level
  # body/markdown_source fields). Allowed when the target is the agent's
  # owner, has previously messaged the agent, or the agent holds the
  # dm_anyone capability. The flow lives in Agents::DirectMessages,
  # shared with the MCP open_dm tool.
  def create
    no_store_response!

    result = Agents::DirectMessages.open_and_post(
      agent: Current.agent,
      user_id: params[:user_id],
      attributes: dm_message_attributes,
      drive_file_ids: dm_drive_file_ids
    )

    if result.ok?
      render json: {
        room: { id: result.payload[:room].id, name: result.payload[:room].name, direct: true },
        message: message_payload(result.payload[:message]),
        thread_id: result.payload[:message].thread_id
      }, status: :created
    elsif result.status == :not_found
      head :not_found
    elsif result.payload
      render json: result.payload, status: result.status
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

    def dm_message_attributes
      if params[:message].is_a?(ActionController::Parameters)
        params.require(:message).permit(
          :body, :client_message_id, :markdown_source,
          :reply_to_message_id, :reply_notify_author
        ).to_h.symbolize_keys
      else
        params.permit(:body, :client_message_id, :markdown_source).to_h.symbolize_keys
      end
    end

    def dm_drive_file_ids
      source = params[:message].is_a?(ActionController::Parameters) ? params[:message] : params
      return :absent unless source.key?(:drive_file_ids)

      raw = source[:drive_file_ids]
      return nil unless raw.is_a?(Array)

      raw.map { |id| id.to_s.strip }.reject(&:blank?).uniq
    end
end
