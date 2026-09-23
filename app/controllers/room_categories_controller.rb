class RoomCategoriesController < ApplicationController
  def create
    Current.user.room_categories.create(room_category_params.merge(position: RoomCategory.next_position_for(Current.user)))
    redirect_to user_sidebar_url
  end

  def update
    category.update(room_category_params)
    redirect_to user_sidebar_url
  end

  def destroy
    category.destroy!
    redirect_to user_sidebar_url
  end

  private
    def category
      Current.user.room_categories.find(params[:id])
    end

    def room_category_params
      params.require(:room_category).permit(:name, :collapsed)
    end
end
