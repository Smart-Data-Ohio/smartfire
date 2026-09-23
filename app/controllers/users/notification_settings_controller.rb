class Users::NotificationSettingsController < ApplicationController
  def update
    @user = Current.user
    @user.assign_attributes(notification_params)

    keywords_saved = true
    if params[:user].key?(:keyword_alerts)
      keywords_saved = @user.replace_keyword_alerts(params[:user][:keyword_alerts])
    end

    if keywords_saved && @user.save
      redirect_to user_profile_url, notice: "✓"
    else
      set_memberships
      render "users/profiles/show", status: :unprocessable_entity
    end
  end

  private
    def notification_params
      params.require(:user).permit(:dnd_enabled, :quiet_hours_enabled, :quiet_hours_start, :quiet_hours_end)
    end

    def set_memberships
      @direct_memberships, @shared_memberships =
        Current.user.memberships.with_ordered_room.partition { |m| m.room.direct? }
    end
end
