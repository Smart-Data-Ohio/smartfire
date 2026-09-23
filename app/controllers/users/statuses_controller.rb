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
      reconcile_meeting_status
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
      params.require(:user).permit(:presence_setting, :custom_status_emoji, :custom_status_text, :custom_status_expires_in,
        :meeting_status_enabled)
    end

    # Opting into meeting status fetches the first busy intervals right
    # away instead of waiting for the 15-minute sweep; opting out drops
    # the cached intervals and clears the label on open profile pages
    # and cards immediately. The opt-in flag is already off, so the
    # re-rendered badge reads through the cleared state without a reload.
    def reconcile_meeting_status
      return unless @user.saved_change_to_meeting_status_enabled?

      if @user.meeting_status_enabled?
        Calendar::MeetingRefreshJob.perform_later(@user.id)
      elsif @user.meeting_cache
        @user.meeting_cache.destroy!
        Calendar::MeetingDispatcher.broadcast_badges_for(@user)
      end
    end

    def set_memberships
      @direct_memberships, @shared_memberships =
        Current.user.memberships.with_ordered_room.partition { |m| m.room.direct? }
    end
end
