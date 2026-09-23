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
    # The recipients mirror Room#unread_memberships exactly: members who are
    # connected, invisible, or muted without a mention keep no server-side
    # unread, so telling their sidebar otherwise would paint a lie.
    def broadcast_unread_room
      unread_user_ids.each do |user_id|
        ActionCable.server.broadcast UnreadRoomsChannel.stream_name_for(user_id), { roomId: room.id }
      end
    end

    def unread_user_ids
      recipients = room.memberships.visible.disconnected.where.not(user: creator)
      ids = recipients.where.not(involvement: :muted).pluck(:user_id)

      muted_recipients = recipients.where(involvement: :muted)
      if muted_recipients.exists?
        ids.concat(muted_recipients.where(user_id: mentionees.select(:id)).pluck(:user_id))
      end
      ids
    end
end
