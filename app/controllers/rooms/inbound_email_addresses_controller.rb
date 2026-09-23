class Rooms::InboundEmailAddressesController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_emailable_room
  before_action :ensure_can_administer_room

  # Issues the room's first secret forward-to address, or rotates it.
  # Anyone who can administer the room (an administrator, or its creator)
  # may rotate; direct and board rooms never have an address.
  def create
    @room.regenerate_inbound_email_token!

    redirect_to edit_room_path(@room), notice: "Room email address rotated."
  end

  private
    def ensure_emailable_room
      head :not_found unless @room.emailable?
    end

    def ensure_can_administer_room
      head :forbidden unless Current.user.can_administer?(@room)
    end
end
