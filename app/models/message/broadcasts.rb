module Message::Broadcasts
  def broadcast_create
    broadcast_append_to message_stream_target, :messages, target: [ message_stream_target, :messages ]
    broadcast_unread_room unless thread_message? || system_note?
  end

  def broadcast_reactions_replace
    # The reactor tooltip reads every boost's booster. Reset first: the
    # toggle above created or destroyed a row, and a preloaded collection
    # would re-render it stale.
    boosts.reset
    ActiveRecord::Associations::Preloader.new(records: [ self ], associations: { boosts: :booster }).call
    broadcast_replace_to conversation, :messages,
      target: ActionView::RecordIdentifier.dom_id(self, :boosts),
      partial: "messages/boosts/reactions", attributes: { maintain_scroll: true }
  end

  def broadcast_remove
    broadcast_remove_to message_stream_target, :messages
  end

  private
    # Fanned out to the room's members rather than published on one global stream, so
    # that the timing of activity in a room only reaches people who are in it.
    # Muted members without a mention are left out, matching
    # Room#unread_memberships: their sidebar must not go unread, and the
    # client paints the badge straight from this broadcast.
    def broadcast_unread_room
      unread_user_ids.each do |user_id|
        ActionCable.server.broadcast UnreadRoomsChannel.stream_name_for(user_id), { roomId: room.id }
      end
    end

    def unread_user_ids
      user_involvements = room.memberships.pluck(:user_id, :involvement)
      return user_involvements.map(&:first) unless user_involvements.any? { |_, involvement| involvement == "muted" }

      mentioned_ids = mentionees.ids
      user_involvements.filter_map do |user_id, involvement|
        user_id unless involvement == "muted" && !mentioned_ids.include?(user_id)
      end
    end
end
