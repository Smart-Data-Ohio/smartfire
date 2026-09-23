class Fizzy::FetchCardJob < ApplicationJob
  # The job records every failure on the viewer's cache row instead of
  # raising, so a missing record is the only thing left to discard: the
  # card or user was deleted after the job was enqueued and there is
  # nothing to update.
  discard_on ActiveJob::DeserializationError

  # Failures are recorded on the cache row for the card frame to render,
  # so there is nothing a retry would fix. attempts: 1 opts out of the
  # inherited transient retries.
  retry_on(*ApplicationJob::TRANSIENT_ERRORS, attempts: 1)

  # Fetches one card with one viewer's own Fizzy token and stores the
  # payload on their cache row. A 404 or 403 means the viewer cannot
  # access the card and is stored as not-found (the frame renders the
  # minimal link chip); anything else is stored as a renderable error.
  def perform(card, user)
    account = user.fizzy_connected_account
    return unless account&.usable?

    cache = Fizzy::CardCache.for_viewer(card: card, user: user)

    begin
      payload = Fizzy::Client.new(token: account.access_token).card(card.account_id, card.number)
    rescue Fizzy::Client::NotFound, Fizzy::Client::Forbidden
      cache.update!(payload: nil, fetched_at: Time.current, fetch_error: Fizzy::CardCache::NOT_FOUND)
      card.broadcast_card_updates
      return
    rescue Fizzy::Client::Unauthorized
      account.mark_disconnected!("Fizzy rejected the linked token (401)")
      cache.update!(payload: nil, fetched_at: nil, fetch_error: nil)
      card.broadcast_card_updates
      return
    rescue Fizzy::Client::Error => error
      cache.update!(fetched_at: Time.current, fetch_error: error.message)
      card.broadcast_card_updates
      return
    rescue ActiveRecord::Encryption::Errors::Decryption
      account.mark_disconnected!(FizzyConnectedAccount::UNREADABLE_TOKEN_REASON)
      card.broadcast_card_updates
      return
    end

    cache.update!(payload: payload, fetched_at: Time.current, fetch_error: nil)
    card.broadcast_card_updates
  end
end
