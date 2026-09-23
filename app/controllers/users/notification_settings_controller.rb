class Users::NotificationSettingsController < ApplicationController
  def update
    @user = Current.user
    @user.assign_attributes(notification_params)
    reconcile_dnd_timer if params[:user]&.key?(:dnd_enabled)

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
      params.require(:user).permit(:dnd_enabled, :quiet_hours_enabled, :quiet_hours_start, :quiet_hours_end,
        :meeting_dnd_enabled, :ooo_notify_enabled)
    end

    # The settings switch means indefinite on/off, while /dnd can set a
    # timer: turning the switch off clears a running timer, and turning
    # it on past an expired timer starts DND fresh instead of staying
    # expired. A save that leaves the switch on preserves a
    # still-running timer.
    def reconcile_dnd_timer
      if ActiveModel::Type::Boolean.new.cast(params[:user][:dnd_enabled])
        @user.dnd_until = nil unless @user.manual_dnd_active?
      else
        @user.dnd_until = nil
      end
    end

    def set_memberships
      @direct_memberships, @shared_memberships =
        Current.user.memberships.with_ordered_room.partition { |m| m.room.direct? }
    end
end
