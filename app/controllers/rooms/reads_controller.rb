class Rooms::ReadsController < ApplicationController
  include RoomScoped

  # Mark the room read: clears the unread flag and advances the unread
  # pointer to the newest root message (see Membership#read). Broadcast
  # on the reads stream like PresenceChannel, so every session updates.
  def create
    @membership.read
    ActionCable.server.broadcast "user_#{Current.user.id}_reads", { room_id: @room.id }

    render json: { room_id: @room.id, unread: false }
  end

  # Mark the room unread starting at a root message: the unread pointer
  # moves to just before it, so the divider lands above it. Other
  # sessions learn it over the unread stream; the requesting tab marks
  # its own row, since the stream skips the current room.
  def destroy
    message = @room.root_messages.find(params[:message_id])
    @membership.mark_unread_before(message)
    ActionCable.server.broadcast UnreadRoomsChannel.stream_name_for(Current.user.id), { roomId: @room.id }

    render json: { room_id: @room.id, unread: true, first_unread_message_id: message.id }
  end
end
