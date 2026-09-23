class Message::QuoteCardsRefreshJob < ApplicationJob
  # Refreshes the quote cards pointing at an edited source message, in
  # whatever room or thread each quoting message lives. Each card
  # re-renders from its quoting message, so per-viewer access still
  # applies: cross-room quotes broadcast as frame placeholders that
  # reload through the quote endpoint.
  #
  # Batched and capped so one viral source cannot stall the queue:
  # quoting messages beyond MAX_QUOTING_MESSAGES keep their stale card
  # until the next load, when the quote stamp in the fragment cache key
  # (which always covers the source edit) serves a fresh fragment.
  # Idempotent: re-rendering a card twice changes nothing.
  BATCH_SIZE = 100
  MAX_QUOTING_MESSAGES = 200

  def perform(source_message_id, max_messages: MAX_QUOTING_MESSAGES, batch_size: BATCH_SIZE)
    message_ids = MessageReference.where(referenced_message_id: source_message_id)
      .order(:id).limit(max_messages).pluck(:message_id)

    Message.where(id: message_ids).find_each(batch_size: batch_size, &:broadcast_quote_cards_replace)
  end
end
