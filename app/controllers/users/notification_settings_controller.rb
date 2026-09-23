class Users::NotificationSettingsController < ApplicationController
  def update
    @user = Current.user
    @user.assign_attributes(notification_params)

    saved = User.transaction do
      keywords_saved = if params[:user]&.key?(:keyword_alerts)
        @user.replace_keyword_alerts(params[:user][:keyword_alerts])
      else
        true
      end

      if keywords_saved && @user.save
        true
      else
        raise ActiveRecord::Rollback
      end
    end

    if saved
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
