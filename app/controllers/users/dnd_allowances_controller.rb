class Users::DndAllowancesController < ApplicationController
  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :set_allowed_user

  # Star someone so their messages still push while DND is on.
  def create
    Current.user.dnd_allowed_users.find_or_create_by!(allowed_user: @allowed_user)

    redirect_to user_url(@allowed_user), notice: "✓"
  end

  def destroy
    Current.user.dnd_allowed_users.where(allowed_user: @allowed_user).delete_all

    redirect_to user_url(@allowed_user), notice: "✓"
  end

  private
    def set_allowed_user
      @allowed_user = User.active.without_bots.find(params[:user_id])
      head :unprocessable_entity if @allowed_user == Current.user
    end
end
