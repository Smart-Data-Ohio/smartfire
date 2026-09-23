# Serves the per-viewer card frame for a Fizzy card referenced in the
# room: the card when the viewer's own token could fetch it, the minimal
# link chip when they have no connected account or Fizzy answers 404/403,
# and a loading or error state otherwise. RoomScoped 404s non-members;
# the card must be referenced in the room through the given message.
class Rooms::Fizzy::CardsController < ApplicationController
  include RoomScoped

  def show
    @card = Fizzy::Card.find(params[:id])
    @message = @room.messages.find(params[:message_id])
    raise ActiveRecord::RecordNotFound unless @card.card_references.exists?(message_id: @message.id)
    @frame_id = helpers.fizzy_card_frame_id(@card, message_id: @message.id)

    unless Current.user.fizzy_connected_account&.usable?
      @connect_required = true
      return render layout: false
    end

    @cache = Fizzy::CardCache.for_viewer(card: @card, user: Current.user)
    request_card_refresh(@card, @cache) if @cache.stale?

    render layout: false
  end

  private
    # Enqueue a refresh for a stale cache row. The claim admits at most
    # one enqueue per viewer and card per staleness window across
    # renders; the fetch result broadcasts fresh frames when it lands.
    def request_card_refresh(card, cache)
      return unless cache.claim_fetch_request!

      begin
        Fizzy::FetchCardJob.perform_later(card, Current.user)
      rescue Redis::BaseError, RedisClient::Error => error
        # The queue is down: serve the stale frame instead of breaking the
        # page, and release the claim so the next render retries.
        cache.release_fetch_request!
        Rails.logger.warn "Skipping Fizzy refresh enqueue for #{card.id}: #{error.class}"
      end
    end
end
