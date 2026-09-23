module Message::Broadcasts
  def broadcast_create
    broadcast_append_to message_stream_target, :messages, target: [ message_stream_target, :messages ]
    broadcast_unread_room unless thread_message? || system_note?
  end

  def broadcast_reactions_replace
    broadcast_replace_to conversation, :messages,
      target: ActionView::RecordIdentifier.dom_id(self, :boosts),
      partial: "messages/boosts/reactions", attributes: { maintain_scroll: true }
  end

  def broadcast_remove
    broadcast_remove_to message_stream_target, :messages
  end

  # Replaces this message's quote-card container, used when a quoted
  # source is edited or deleted. Rendering preloads first so the cards
  # read sources from memory, like reply tombstone broadcasts do.
  def broadcast_quote_cards_replace
    self.class.preload_rendering_details([ self ])
    Message::MentionPreloader.preload_for([ self ])

    broadcast_replace_to message_stream_target, :messages,
      target: ActionView::RecordIdentifier.dom_id(self, :message_link_cards),
      partial: "messages/message_links/cards", locals: { message: self },
      attributes: { maintain_scroll: true }
  end

  private
    # Fanned out to the room's members rather than published on one global stream, so
    # that the timing of activity in a room only reaches people who are in it.
    def broadcast_unread_room
      room.memberships.pluck(:user_id).each do |user_id|
        ActionCable.server.broadcast UnreadRoomsChannel.stream_name_for(user_id), { roomId: room.id }
      end
    end
end
