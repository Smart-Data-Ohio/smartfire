class Messages::ByBotsController < MessagesController
  include AgentAuthorization, RawRequestBody

  allow_bot_access only: %i[ index create update destroy ]
  skip_before_action :ensure_can_edit, :ensure_can_delete

  before_action :set_room
  before_action :set_message, only: %i[ update destroy ]
  before_action :ensure_can_manage_bot_message, only: %i[ update destroy ]
  require_agent_capability :post_messages, only: %i[ create update destroy ]
  require_agent_capability :read_messages, only: :index
  before_action :ensure_body_or_attachment_present, only: :create

  def index
    @messages = find_paged_messages
    set_pagination_headers
  end

  def create
    # Boards hold posts, not root messages; agents reply inside posts from
    # the next slice on.
    return head :unprocessable_content if @room.board?

    # Agent-backed bots post through the shared agent posting service, so
    # the bot-key route hits the same daily message budget as the agent
    # token API and MCP. Legacy bots without an agent row keep the frozen
    # human-equivalent path.
    if (agent = Current.user.agent)
      post_as_agent(agent)
    else
      super
      head :created, location: message_url(@message)
    end
  end

  def destroy
    super
    head :no_content
  end

  private
    def post_as_agent(agent)
      result = Agents::Posting.post(
        agent: agent, room: @room,
        attributes: agent_posting_attributes,
        drive_file_ids: :absent
      )

      if result.ok?
        @message = result.payload
        head :created, location: message_url(@message)
      elsif result.status == :not_found
        head :not_found
      elsif result.payload
        render json: result.payload, status: result.status
      else
        render json: result.failure_body, status: result.status
      end
    end

    # The bot posting API takes a raw text body or an attachment file, never
    # Drive ids or Markdown; client_message_id rides along when a caller
    # sends the message object form, so a retried create returns the
    # original without burning budget.
    def agent_posting_attributes
      attrs = message_params.to_h.symbolize_keys
      attrs[:client_message_id] ||= params.dig(:message, :client_message_id)
      attrs.delete(:client_message_id) if attrs[:client_message_id].nil?
      attrs
    end

    # The bot posting API takes no Drive attachments (docs/google-drive.md).
    def drive_file_ids_key_present?
      false
    end

    # This endpoint renders message_payload, which never reads boosts or image
    # variant records, so it skips the heavier rendering preloads.
    def paged_message_scope
      @room.root_messages.with_payload_details
    end

    def set_room
      @room = Current.user.rooms.find_by(id: params[:room_id])

      head :not_found unless @room
    end

    def ensure_body_or_attachment_present
      if params[:attachment].blank? && raw_request_body.blank?
        head :unprocessable_content
      end
    end

    def ensure_can_manage_bot_message
      head :forbidden if @message.system_note? || !Current.user.can_administer?(@message)
    end

    def set_pagination_headers
      headers["X-Total-Count"] = @room.root_messages.count.to_s

      if next_page = next_page_params
        headers["Link"] = %(<#{room_bot_messages_url(@room, params[:bot_key], **next_page)}>; rel="next")
      end
    end

    def next_page_params
      if @messages.any?
        if params[:after].present?
          { after: @messages.last.id } if @room.root_messages.after(@messages.last).exists?
        else
          { before: @messages.first.id } if @room.root_messages.before(@messages.first).exists?
        end
      end
    end

    def message_params
      attributes = if params[:attachment]
        params.permit(:attachment)
      else
        { body: raw_request_body }
      end

      attributes.merge(markdown_source: nil)
    end
end
