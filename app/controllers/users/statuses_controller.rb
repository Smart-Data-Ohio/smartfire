class Users::StatusesController < ApplicationController
  def update
    @user = Current.user
    @user.assign_attributes(status_params)

    if params[:user]&.dig(:clear_custom_status).present?
      @user.custom_status_emoji = nil
      @user.custom_status_text = nil
      @user.custom_status_expires_at = nil
    end

    if @user.save
      redirect_to user_profile_url, notice: "✓"
    else
      set_memberships
      render "users/profiles/show", status: :unprocessable_entity
    end
  rescue ArgumentError
    @user ||= Current.user
    @user.errors.add(:custom_status_expires_in, "is not valid")
    set_memberships
    render "users/profiles/show", status: :unprocessable_entity
  end

  private
    def status_params
      params.require(:user).permit(:presence_setting, :custom_status_emoji, :custom_status_text, :custom_status_expires_in)
    end

    def set_memberships
      @direct_memberships, @shared_memberships =
        Current.user.memberships.with_ordered_room.partition { |m| m.room.direct? }
    end
end
