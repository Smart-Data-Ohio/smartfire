class Users::StatusesController < ApplicationController
  def update
    @user = Current.user
    @user.assign_attributes(status_params)

    if params[:user]&.dig(:clear_custom_status).present?
      @user.custom_status_emoji = nil
      @user.custom_status_text = nil
      @user.custom_status_expires_at = nil
    end

    return render_ooo_error(@ooo_error) if apply_ooo_params == false

    if @user.save
      reconcile_meeting_status
      reconcile_ooo_calendar
      broadcast_manual_ooo_change
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
        :meeting_status_enabled, :ooo_calendar_enabled)
    end

    # A preset sets the manual OOO end (plus the note); clearing drops
    # both; otherwise a submitted note edits the note alone. Returns false
    # with @ooo_error set when the preset is unknown or its date is blank,
    # unparseable, or past — nothing is saved then, so the form re-renders
    # with the member's other edits intact.
    def apply_ooo_params
      user_params = params[:user] || {}

      if user_params[:clear_ooo].present?
        @user.ooo_until = nil
        @user.ooo_note = nil
      elsif user_params[:ooo_preset].present?
        preset = user_params[:ooo_preset].to_s

        unless User::StatusSettings::OOO_PRESETS.include?(preset)
          @ooo_error = "is not valid"
          return false
        end

        ooo_until = @user.ooo_preset_until(preset, user_params[:ooo_until_custom])

        if ooo_until.nil? || ooo_until <= Time.current
          @ooo_error = "needs a future date and time"
          return false
        end

        @user.ooo_until = ooo_until
        @user.ooo_note = user_params[:ooo_note].presence
      elsif user_params.key?(:ooo_note)
        @user.ooo_note = user_params[:ooo_note].presence
      end

      true
    end

    def render_ooo_error(message)
      @user.errors.add(:ooo_until, message)
      set_memberships
      render "users/profiles/show", status: :unprocessable_entity
    end

    # Opting into meeting status fetches the first busy intervals right
    # away instead of waiting for the 15-minute sweep; opting out drops
    # the cached intervals and clears the label on open profile pages
    # and cards immediately. The opt-in flag is already off, so the
    # re-rendered badge reads through the cleared state without a reload.
    # The cache row is shared with calendar OOO: opting out while that is
    # still on clears only the busy intervals.
    def reconcile_meeting_status
      return unless @user.saved_change_to_meeting_status_enabled?

      if @user.meeting_status_enabled?
        Calendar::MeetingRefreshJob.perform_later(@user.id)
      elsif (cache = @user.meeting_cache)
        if @user.ooo_calendar_enabled?
          cache.update!(busy_intervals: [])
        else
          cache.destroy!
        end
        Calendar::MeetingDispatcher.broadcast_badges_for(@user)
      end
    end

    # Same shape as meeting status for the calendar-OOO opt-in: an
    # immediate first fetch on the way in, cleared intervals plus a live
    # badge-and-notice update on the way out. The row survives while
    # meeting status still wants it.
    def reconcile_ooo_calendar
      return unless @user.saved_change_to_ooo_calendar_enabled?

      if @user.ooo_calendar_enabled?
        Calendar::MeetingRefreshJob.perform_later(@user.id)
      elsif (cache = @user.meeting_cache)
        if @user.meeting_status_enabled?
          cache.update!(ooo_intervals: [])
        else
          cache.destroy!
        end
        # Claim first: the broadcast below announces the cleared state,
        # so a later re-opt-in must find stored false to announce again.
        @user.claim_ooo_broadcast!(@user.out_of_office?)
        Calendar::OooDispatcher.broadcast_ooo_for(@user)
      end
    end

    # A manual OOO set or cleared (or a note edited) updates open badges
    # and DM notices at once; the claim keeps the next sweep from
    # announcing the same flip again. The broadcast runs even when the
    # claim loses (a note edit, or a clear while calendar OOO still
    # covers), since the rendered text changed.
    def broadcast_manual_ooo_change
      return unless @user.saved_change_to_ooo_until? || @user.saved_change_to_ooo_note?

      @user.claim_ooo_broadcast!(@user.out_of_office?)
      Calendar::OooDispatcher.broadcast_ooo_for(@user)
    end

    def set_memberships
      @direct_memberships, @shared_memberships =
        Current.user.memberships.with_ordered_room.partition { |m| m.room.direct? }
    end
end
