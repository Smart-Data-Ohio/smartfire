class MessageEmbedSuppressionsController < ApplicationController
  before_action :set_source

  # The author removes the link embeds (generic and LinkedIn) from their
  # message. The references stay, but the cards stop rendering; a later
  # edit still re-syncs them for a potential restore.
  def create
    head :forbidden and return unless Current.user == @source.creator
    head :forbidden and return if @source.thread_message? && @source.thread.locked?

    @source.update!(embeds_suppressed: true) unless @source.embeds_suppressed?
    broadcast_card_removal

    no_store_response!
    respond_to do |format|
      format.turbo_stream do
        render turbo_stream: [
          turbo_stream.replace([ @source, :linkedin_cards ],
            partial: "linkedin/posts/cards", locals: { message: @source }),
          turbo_stream.replace([ @source, :link_embed_cards ],
            partial: "link_embeds/cards", locals: { message: @source })
        ]
      end
      format.json { render json: { embeds_suppressed: true } }
      format.html { redirect_to message_permalink_url(@source) }
    end
  end

  private
    def set_source
      if params[:room_id].present?
        room = Current.user.rooms.find(params[:room_id])
        @source = if params[:thread_id].present?
          room.channel_threads.find(params[:thread_id]).messages.find(params[:message_id])
        else
          room.root_messages.find(params[:message_id])
        end
      else
        @source = Current.user.reachable_messages.find(params[:message_id])
      end
    end

    def broadcast_card_removal
      @source.broadcast_replace_to @source.message_stream_target, :messages,
        target: [ @source, :linkedin_cards ], partial: "linkedin/posts/cards", attributes: { maintain_scroll: true }
      @source.broadcast_replace_to @source.message_stream_target, :messages,
        target: [ @source, :link_embed_cards ], partial: "link_embeds/cards", attributes: { maintain_scroll: true }
    end
end
