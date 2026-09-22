module RoomScoped
  extend ActiveSupport::Concern

  included do
    before_action :set_room
  end

  private
    def set_room
      @membership = Current.user.memberships.joins(:room).merge(Room.alive).find_by!(room_id: params[:room_id])
      @room = @membership.room
    end
end
