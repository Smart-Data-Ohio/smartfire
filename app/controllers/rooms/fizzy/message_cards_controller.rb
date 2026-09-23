# Creates a Fizzy card from a room or thread message, opened from the
# message actions menu. The board picker lists the member's own Fizzy
# boards, the title is prefilled from the message's first line, and the
# description from its text plus a permalink back. Creation runs with
# the member's own linked Fizzy token, then posts a reply carrying the
# new card's URL so it unfurls in the conversation.
class Rooms::Fizzy::MessageCardsController < ApplicationController
  include RoomScoped, Messages::BotWebhooks

  TITLE_PREFILL_CHARS = 120

  before_action :set_conversation_and_message

  def new
    @account = Current.user.fizzy_connected_account
    unless @account&.usable?
      return render :new
    end

    @boards = fetch_boards(@account)
    return if performed?

    @board_id = params[:board_id]
    @title = params[:title] || @message.plain_text_body.each_line.first.to_s.strip.truncate(TITLE_PREFILL_CHARS)
    @description = params[:description] || default_description
  end

  def create
    @account = Current.user.fizzy_connected_account
    unless @account&.usable?
      return redirect_to user_profile_path, alert: "Connect Fizzy on your profile first."
    end

    # Check before any Fizzy call: creating the card first would leave an
    # orphan in Fizzy when the reply cannot be posted.
    if @thread&.locked?
      return redirect_to conversation_path, alert: "This thread is locked"
    end

    @board_id = params[:board_id].to_s
    @title = params[:title].to_s.strip
    @description = params[:description].to_s

    if @board_id.blank? || @title.blank?
      @boards = fetch_boards(@account)
      return if performed?

      flash.now[:alert] = "Choose a board and enter a title."
      return render :new, status: :unprocessable_entity
    end

    begin
      card = client(@account).create_card(@account.fizzy_account_id, @board_id, title: @title, description: @description.presence)
    rescue Fizzy::Client::Unauthorized
      return handle_create_unauthorized(@account)
    rescue Fizzy::Client::Refused, Fizzy::Client::Error => error
      return redirect_to conversation_path, alert: "Fizzy refused the new card (#{error.message})."
    rescue ActiveRecord::Encryption::Errors::Decryption
      @account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
      return redirect_to user_profile_path, alert: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON
    end

    post_created_reply(card)
    redirect_to conversation_path, notice: "Fizzy card ##{card["number"]} created."
  rescue ChannelThread::LockedError => error
    redirect_to conversation_path, alert: error.message
  rescue ActiveRecord::RecordInvalid => error
    redirect_to conversation_path,
      alert: "Fizzy card ##{card["number"]} created, but the reply could not be posted (#{error.record.errors.full_messages.to_sentence})."
  end

  private
    def set_conversation_and_message
      if params[:thread_id].present?
        @thread = @room.channel_threads.find(params[:thread_id])
        @message = @thread.messages.find(params[:message_id])
      else
        @message = @room.root_messages.find(params[:message_id])
      end
    end

    def conversation_path
      if @thread
        room_thread_path(@room, @thread)
      else
        room_path(@room)
      end
    end

    def client(account)
      Fizzy::Client.new(token: account.access_token)
    end

    def default_description
      [ @message.plain_text_body.presence, "Source: #{helpers.message_link_url(@message)}" ].compact.join("\n\n")
    end

    def fetch_boards(account)
      client(account).boards(account.fizzy_account_id)
    rescue Fizzy::Client::Unauthorized
      account.mark_disconnected!("Fizzy rejected the linked token (401)")
      redirect_to user_profile_path, alert: "Fizzy rejected the linked token. Reconnect on your profile."
      nil
    rescue Fizzy::Client::Error
      redirect_to conversation_path, alert: "Could not reach Fizzy. Try again."
      nil
    rescue ActiveRecord::Encryption::Errors::Decryption
      account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
      redirect_to user_profile_path, alert: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON
      nil
    end

    # A 401 on create is ambiguous: the token may be revoked, or it may
    # be a read-only token (Fizzy's read permission rejects writes with
    # 401). A cheap identity read tells them apart: when it succeeds the
    # token is valid but lacks write permission, so the account stays
    # connected and the member learns what to fix. Only a rejected probe
    # disconnects: when the probe itself errors (a 500, a timeout), the
    # token may be fine, so the account stays connected and the member
    # retries. A genuinely dead token disconnects on the next read's 401.
    def handle_create_unauthorized(account)
      account.access_token # raise decryption now if unreadable
      client(account).identity
      redirect_to conversation_path, alert: "That Fizzy token is read-only. Generate a Read + Write token to create cards."
    rescue Fizzy::Client::Unauthorized
      account.mark_disconnected!("Fizzy rejected the linked token (401)")
      redirect_to user_profile_path, alert: "Fizzy rejected the linked token. Reconnect on your profile."
    rescue Fizzy::Client::Error
      redirect_to conversation_path, alert: "Could not reach Fizzy to verify the token. Try again."
    rescue ActiveRecord::Encryption::Errors::Decryption
      account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
      redirect_to user_profile_path, alert: FizzyConnectedAccount::UNREADABLE_TOKEN_REASON
    end

    def post_created_reply(card)
      url = card["url"].presence || "#{Fizzy::Client.api_base_url}/#{@account.fizzy_account_id}/cards/#{card["number"]}"
      source = "Created from #{helpers.message_link_url(@message)}:\n#{url}"

      reply = if @thread
        @thread.post_message!(creator: Current.user, attributes: { markdown_source: source, reply_to_message_id: @message.id })
      else
        @room.root_messages.create!(creator: Current.user, markdown_source: source, reply_to_message: @message)
      end
      reply.broadcast_create
      deliver_webhooks_to_bots(reply) unless @thread
      reply
    end
end
