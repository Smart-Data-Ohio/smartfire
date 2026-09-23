class Rooms::ReadsController < ApplicationController
  include RoomScoped

  # Mark the room read: clears the unread flag and advances the unread
  # pointer to the newest root message (see Membership#read).
  def create
    @membership.read
    render json: { room_id: @room.id, unread: false }
  end

  # Mark the room unread starting at a root message: the unread pointer
  # moves to just before it, so the divider lands above it.
  def destroy
    message = @room.root_messages.find(params[:message_id])
    @membership.mark_unread_before(message)

    render json: { room_id: @room.id, unread: true, first_unread_message_id: message.id }
  end
end
