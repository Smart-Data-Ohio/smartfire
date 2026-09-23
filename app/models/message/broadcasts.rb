module Message::Broadcasts
  def broadcast_create
    broadcast_append_to message_stream_target, :messages, target: [ message_stream_target, :messages ]
    broadcast_unread_room unless thread_message?
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
      ids = room.memberships.pluck(:user_id)
      return ids unless room.memberships.where(involvement: :muted).exists?

      ids - room.memberships.where(involvement: :muted).where.not(user_id: mentionees.select(:id)).pluck(:user_id)
    end
end
