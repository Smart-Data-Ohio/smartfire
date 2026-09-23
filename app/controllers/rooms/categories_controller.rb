class Rooms::CategoriesController < ApplicationController
  include RoomScoped

  # Assign the room to one of the member's categories, or unassign it
  # when room_category_id is blank. Categories hold channels only.
  def update
    unless @room.open? || @room.closed?
      head :unprocessable_content
      return
    end

    category = if params[:room_category_id].present?
      Current.user.room_categories.find(params[:room_category_id])
    end
    @membership.update!(room_category: category)

    head :ok
  end
end
