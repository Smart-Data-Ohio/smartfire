class MessagesController < ApplicationController
  include ActiveStorage::SetCurrent, RoomScoped, Messages::DriveAttachable

  before_action :set_room, except: :create
  before_action :set_message, only: %i[ show edit update destroy actions ]
  before_action :ensure_can_edit, only: %i[ edit update ]
  before_action :ensure_can_delete, only: :destroy

  layout false, only: :index

  def index
    no_store_response! if request.format.json?
    # The page is selected without its associations so that a conditional GET
    # can be answered from ids and timestamps alone. Only a request that is
    # actually going to render pays to load bodies, attachments and boosts.
    @messages = find_paged_messages

    if @messages.any?
      fresh_when @messages, etag: [ @messages, rendered_related_stamp(@messages) ]
      unless performed?
        Message.preload_rendering_details(@messages)
        Message::MentionPreloader.preload_for(@messages)
      end
    else
      head :no_content
    end
  end

  def create
    set_room
    @message = @room.root_messages.new(message_params)
    apply_drive_file_ids!(@message) if drive_file_ids_key_present?
    @message.save!
    @message.process_attachment

    @message.broadcast_create
    deliver_webhooks_to_bots
  rescue ActiveRecord::RecordNotFound
    render action: :room_not_found
  rescue ActiveRecord::RecordInvalid => error
    render_record_invalid(error)
  end

  def show
    no_store_response! if request.format.json?
  end

  def preview
    source = params.require(:message).permit(:markdown_source).fetch(:markdown_source)

    if source.length > Message::Markdown::SOURCE_LIMIT
      render json: { error: "Markdown is limited to #{Message::Markdown::SOURCE_LIMIT.to_fs(:delimited)} characters" }, status: :unprocessable_content
    else
      content = ActionText::Content.new(Message::Markdown.render(source, room: @room))
      render json: { html: view_context.markdown_message_presentation(content) }
    end
  end

  def actions
    no_store_response!
    render json: { actions: message_actions_payload(@message) }
  end

  def edit
  end

  def update
    attributes = message_params
    @message.preserve_legacy_attachments_on_next_markdown_render! if !@message.markdown? && attributes[:markdown_source].present?
    @message.assign_attributes(attributes)
    apply_drive_file_ids!(@message) if drive_file_ids_key_present?
    # edited_at is stamped only here (and the thread endpoint), and only
    # when the text itself changed: never by reaction touches, reply
    # tombstones, card fetches, attachment-only or identical saves, so
    # the "(edited)" marker means the author edited after posting.
    @message.edited_at = Time.current if @message.body_content_will_change?
    @message.save!

    @message.broadcast_replace_to @room, :messages, target: [ @message, :presentation ], partial: "messages/presentation", attributes: { maintain_scroll: true }
    @message.broadcast_replace_to @room, :messages, target: [ @message, :meta ], partial: "messages/meta", attributes: { maintain_scroll: true }
    # References re-sync on save (see Message's after_update_commit
    # hooks), so an edit that adds or removes a URL replaces the card
    # containers too. The containers always render, which gives both
    # cases a broadcast target.
    @message.broadcast_replace_to @room, :messages, target: [ @message, :github_pr_cards ], partial: "github/pull_requests/cards", attributes: { maintain_scroll: true }
    @message.broadcast_replace_to @room, :messages, target: [ @message, :twitter_cards ], partial: "twitter/posts/cards", attributes: { maintain_scroll: true }
    if drive_file_ids_key_present?
      @message.broadcast_replace_to @room, :messages, target: [ @message, :drive_attachments ],
        partial: "messages/drive_attachments", locals: { message: @message }, attributes: { maintain_scroll: true }
    end

    respond_to do |format|
      format.html { redirect_to room_message_url(@room, @message) }
      format.json { render json: message_payload(@message) }
    end
  rescue ActiveRecord::RecordInvalid => error
    render_record_invalid(error)
  end

  def destroy
    replies = @message.replies.to_a
    parented_thread = @message.channel_thread
    @message.destroy
    @message.broadcast_remove
    broadcast_reply_tombstones(replies)
    broadcast_thread_summary_refresh(parented_thread) if parented_thread
  end

  private
    def set_message
      @message = @room.root_messages.find(params[:id])
    end

    def ensure_can_edit
      head :forbidden unless Current.user == @message.creator
    end

    def ensure_can_delete
      head :forbidden unless Current.user == @message.creator || Current.user.administrator?
    end


    def find_paged_messages
      case
      when params[:before].present?
        paged_message_scope.page_before(@room.root_messages.find(params[:before]))
      when params[:after].present?
        paged_message_scope.page_after(@room.root_messages.find(params[:after]))
      else
        paged_message_scope.last_page
      end
    end

    # Subclasses that render something other than the message partials override
    # this to preload only what their representation reads.
    def paged_message_scope
      @room.root_messages
    end

    # The newest timestamp over everything the page renders that the
    # messages' own rows don't cover: reply sources (whose edits must
    # refresh previews even when the source scrolled off the page),
    # card rows (whose fetch completion must refresh cards) and
    # creators (whose renames must refresh authors). Aggregate queries
    # only, so a conditional GET still never loads bodies. edited_at is
    # folded in beside updated_at because a legacy body edit rewrites
    # only the rich-text row, leaving the message row untouched.
    def rendered_related_stamp(messages)
      message_ids = messages.map(&:id)
      reply_ids = messages.filter_map(&:reply_to_message_id)

      # Formatted at microsecond precision like collection cache keys:
      # a raw Time expands into an etag at whole seconds, which blinds
      # the etag to same-second changes.
      [
        Message.where(id: message_ids).maximum(:edited_at),
        reply_ids.any? ? Message.where(id: reply_ids).maximum(:updated_at) : nil,
        reply_ids.any? ? Message.where(id: reply_ids).maximum(:edited_at) : nil,
        Github::PullRequestReference.where(message_id: message_ids).joins(:pull_request).maximum("github_pull_requests.updated_at"),
        Twitter::PostReference.where(message_id: message_ids).joins(:post).maximum("twitter_posts.updated_at"),
        EventReference.where(message_id: message_ids).joins(:event).maximum("events.updated_at"),
        User.where(id: messages.map(&:creator_id)).maximum(:updated_at)
      ].compact.max&.utc&.to_fs(:usec)
    end


    def message_params
      permitted = params.require(:message).permit(
        :body, :attachment, :client_message_id, :markdown_source,
        :reply_to_message_id, :reply_notify_author, drive_file_ids: []
      )
      permitted.delete(:drive_file_ids)

      if permitted.key?(:markdown_source) && !permitted[:markdown_source].nil?
        permitted.delete(:body)
      elsif action_name == "update"
        permitted[:markdown_source] = nil
      end

      if permitted.key?(:reply_to_message_id)
        permitted[:reply_to_message_id] = if permitted[:reply_to_message_id].present?
          @room.root_messages.find(permitted[:reply_to_message_id]).id
        end
      end
      if permitted.key?(:reply_notify_author)
        permitted[:reply_notify_author] = ActiveModel::Type::Boolean.new.cast(permitted[:reply_notify_author])
      end

      permitted.to_h.symbolize_keys
    end

    # Deleting a message leaves a tombstone on each reply (see
    # Message#preserve_reply_tombstones); replace the replies in other
    # clients so their previews flip to it. Each reply renders on its
    # own stream: a thread reply can point at the parent room message.
    def broadcast_reply_tombstones(replies)
      replies.each(&:reload)
      Message.preload_rendering_details(replies)

      replies.each do |reply|
        reply.broadcast_replace_to reply.message_stream_target, :messages,
          target: reply, partial: "messages/message", attributes: { maintain_scroll: true }
      end
    end

    # A delete changes the parent thread's summary (message counts,
    # previews) in other clients' thread browsers. The browser reloads
    # on UnreadThreadsChannel messages; refreshOnly gets the reload
    # without marking the thread unread.
    def broadcast_thread_summary_refresh(thread)
      thread.room.memberships.pluck(:user_id).each do |user_id|
        ActionCable.server.broadcast UnreadThreadsChannel.stream_name_for(user_id),
          { threadId: thread.id, roomId: thread.room_id, refreshOnly: true }
      end
    end

    def render_record_invalid(error)
      respond_to do |format|
        format.json { render json: { errors: error.record.errors.to_hash }, status: :unprocessable_content }
        format.any { head :unprocessable_content }
      end
    end


    def deliver_webhooks_to_bots
      # Agent-backed bots are delivered only through Agent::DeliveryJob (see
      # Message::AgentDelivery); the legacy webhook bypasses grant and rate
      # checks, so it serves bots without an Agent row only. The hop limit
      # still applies: a chain that reached it stops here instead of
      # looping through a legacy bot.
      bots = bots_eligible_for_webhook.excluding(@message.creator).where.missing(:agent)
      return if bots.empty?
      return if Agent::Delivery.hop_for_message(@message) >= Agent::Delivery::HOP_LIMIT

      bots.each { |bot| bot.deliver_webhook_later(@message) }
    end

    def bots_eligible_for_webhook
      @room.direct? ? @room.users.active_bots : @message.mentionees.active_bots
    end
end
