class Agents::MessagesController < MessagesController
  include AgentAuthorization
  include AgentApiThrottle

  allow_agent_access only: :create

  # Bearer-only endpoint. Forgery protection stays on: Bearer agent already
  # bypass it through the Authentication concern, and a session-cookie request
  # that trips it gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  # Re-declaring :set_room replaces the inherited except-create callback (same
  # filter name), so membership is checked as a before_action that halts with
  # 404 before authorization runs. Mirrors Messages::ByBotsController.
  before_action :set_room, only: :create
  before_action :ensure_agent_token, only: :create
  require_agent_capability :post_messages, only: :create
  throttle_agent_api limit: 60, only: :create

  # POST /rooms/:room_id/agents/messages (Bearer-only, JSON). Posts a root
  # message, or — with a top-level thread_id — a reply inside that thread
  # through ChannelThread#post_message!. The thread must belong to the room
  # (404 otherwise) and must not be locked (422). The response carries
  # thread_id, null for root messages. The posting flow lives in
  # Agents::Posting, shared with the MCP post_message tool.
  def create
    result = Agents::Posting.post(
      agent: Current.agent,
      room: @room,
      thread_id: params[:thread_id],
      attributes: agent_message_attributes,
      drive_file_ids: agent_drive_file_ids
    )

    if result.ok?
      @message = result.payload
      render json: message_payload(@message).merge(thread_id: @message.thread_id), status: :created
    elsif result.status == :not_found
      head :not_found
    elsif result.payload
      render json: result.payload, status: result.status
    else
      render json: result.failure_body, status: result.status
    end
  rescue ActiveRecord::RecordNotFound
    render action: :room_not_found
  end

  private
    def set_room
      @room = Current.user.rooms.find_by(id: params[:room_id])

      head :not_found unless @room
    end

    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token?
    end

    def reject_session_request
      render json: { error: "Forbidden: Bearer agent token required" }, status: :forbidden
    end

    def agent_message_attributes
      params.require(:message).permit(
        :body, :attachment, :client_message_id, :markdown_source,
        :reply_to_message_id, :reply_notify_author
      ).to_h.symbolize_keys
    end

    # :absent when the form sent no Drive key at all, nil when the key
    # held no usable array (which fails validation, like the human
    # endpoint), otherwise the id array.
    def agent_drive_file_ids
      return :absent unless drive_file_ids_key_present?

      raw = params[:message][:drive_file_ids]
      return nil unless raw.is_a?(Array)

      raw.map { |id| id.to_s.strip }.reject(&:blank?).uniq
    end
end
