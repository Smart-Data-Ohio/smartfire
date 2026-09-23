class ChannelThreadMessagesController < ApplicationController
  include RoomScoped, Messages::DriveAttachable

  before_action :set_thread
  before_action :set_message, only: %i[ show update destroy actions ]
  before_action :ensure_can_edit, only: :update
  before_action :ensure_can_delete, only: :destroy
  before_action :ensure_thread_message_writable, only: :update

  def index
    @messages = Message::MentionPreloader.preload_for(find_paged_messages)
    no_store_response! if request.format.json?

    respond_to do |format|
      format.html
      format.json do
        caching_thread_payloads do
          render json: { messages: @messages.map { |message| message_payload(message, include_thread_summary: false) } }
        end
      end
    end
  end

  def show
    no_store_response! if request.format.json?
    respond_to do |format|
      # A thread message belongs in the parent room shell, where the selected
      # thread has its composer and context panel. Keep this nested URL for
      # JSON/REST operations, but make a human navigation land in that shell.
      format.html { redirect_to room_path(@room, thread: @thread.id, message_id: @message.id) }
      format.json { render json: message_payload(@message, include_thread_summary: false) }
    end
  end

  def actions
    no_store_response!
    render json: { actions: message_actions_payload(@message) }
  end

  def create
    if (duplicate = Message.find_duplicate(room: @room, creator: Current.user, client_message_id: params.dig(:message, :client_message_id)))
      # A retried create: return the original without re-posting.
      @message = duplicate
    else
      @message = @thread.post_message!(creator: Current.user, attributes: message_params, drive_file_ids: validated_drive_file_ids!)
      @message.broadcast_create
    end

    no_store_response! if request.format.json?
    respond_to do |format|
      format.html { redirect_to room_thread_path(@room, @thread) }
      format.json { render json: message_payload(@message, include_thread_summary: false), status: :created }
      format.turbo_stream
    end
  rescue ActiveRecord::RecordInvalid => error
    render_error error.record.errors.full_messages.to_sentence
  rescue ChannelThread::LockedError => error
    render_error error.message, status: :forbidden
  end

  def update
    attributes = message_params
    replace_drive_attachments = drive_file_ids_key_present?
    @thread.with_lock do
      @thread.reload
      raise ChannelThread::LockedError, "This thread is locked" if @thread.locked?

      @message.reload
      @message.preserve_legacy_attachments_on_next_markdown_render! if !@message.markdown? && attributes[:markdown_source].present?
      @message.assign_attributes(attributes)
      apply_drive_file_ids!(@message) if replace_drive_attachments
      # Stamped only by the edit endpoints (see MessagesController), and
      # only when the text itself changed: never by reaction touches,
      # reply tombstones, card fetches, attachment-only or identical saves.
      @message.edited_at = Time.current if @message.body_content_will_change?
      @message.save!
    end
    @message.broadcast_replace_to @thread, :messages,
      target: [ @message, :presentation ], partial: "messages/presentation", attributes: { maintain_scroll: true }
    @message.broadcast_replace_to @thread, :messages,
      target: [ @message, :meta ], partial: "messages/meta", attributes: { maintain_scroll: true }
    # References re-sync on save (see Message's after_update_commit
    # hooks), so an edit that adds or removes a URL replaces the card
    # containers too. The containers always render, which gives both
    # cases a broadcast target.
    @message.broadcast_replace_to @thread, :messages,
      target: [ @message, :github_pr_cards ], partial: "github/pull_requests/cards", attributes: { maintain_scroll: true }
    @message.broadcast_replace_to @thread, :messages,
      target: [ @message, :twitter_cards ], partial: "twitter/posts/cards", attributes: { maintain_scroll: true }
    @message.broadcast_replace_to @thread, :messages,
      target: [ @message, :fizzy_cards ], partial: "fizzy/cards/cards", attributes: { maintain_scroll: true }
    if replace_drive_attachments
      @message.broadcast_replace_to @thread, :messages, target: [ @message, :drive_attachments ],
        partial: "messages/drive_attachments", locals: { message: @message }, attributes: { maintain_scroll: true }
    end

    no_store_response! if request.format.json?
    respond_to do |format|
      format.html { redirect_to room_thread_message_path(@room, @thread, @message) }
      format.json { render json: message_payload(@message, include_thread_summary: false) }
    end
  rescue ActiveRecord::RecordInvalid => error
    render_error error.record.errors.full_messages.to_sentence
  rescue ChannelThread::LockedError => error
    render_error error.message, status: :forbidden
  end

  def destroy
    replies = @message.replies.to_a
    @message.destroy!
    @message.broadcast_remove
    broadcast_reply_tombstones(replies)
    broadcast_thread_summary_refresh(@thread)

    respond_to do |format|
      format.html { redirect_to room_thread_path(@room, @thread) }
      format.any { head :no_content }
    end
  end

  private
    def set_thread
      @thread = @room.channel_threads.find(params[:thread_id])
    end

    def set_message
      @message = @thread.messages.find(params[:id])
    end

    # System notes are immutable timeline entries: nobody edits or deletes
    # them, not even their actor or an administrator.
    def ensure_can_edit
      head :forbidden if @message.system_note? || Current.user != @message.creator
    end

    def ensure_can_delete
      head :forbidden if @message.system_note? || (Current.user != @message.creator && !Current.user.administrator?)
    end

    def ensure_thread_message_writable
      return unless @thread.locked?

      head :forbidden
    end

    def find_paged_messages
      messages = @thread.messages.with_rendering_details
      case
      when params[:before].present?
        messages.page_before(@thread.messages.find(params[:before]))
      when params[:after].present?
        messages.page_after(@thread.messages.find(params[:after]))
      else
        messages.last_page
      end
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

      permitted[:reply_to_message_id] = permitted[:reply_to_message_id].presence if permitted.key?(:reply_to_message_id)
      if permitted.key?(:reply_notify_author)
        permitted[:reply_notify_author] = ActiveModel::Type::Boolean.new.cast(permitted[:reply_notify_author])
      end
      permitted.to_h.symbolize_keys
    end

    # Deleting a message leaves a tombstone on each reply (see
    # Message#preserve_reply_tombstones); replace the replies in other
    # clients so their previews flip to it.
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

    def render_error(message, status: :unprocessable_content)
      respond_to do |format|
        format.html { head status }
        format.json { render json: { error: message }, status: status }
        format.any { head status }
      end
    end
end
