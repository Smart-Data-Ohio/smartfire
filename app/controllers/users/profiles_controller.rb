class Users::ProfilesController < ApplicationController
  before_action :set_user

  def show
    set_memberships
    set_two_factor_devices
  end

  def update
    email_changing = email_change_requested?
    password_changing = params[:user].respond_to?(:key?) && params[:user][:password].present?

    # Check the current password before assigning anything: a submitted new
    # password would otherwise replace the digest it is checked against.
    if email_changing && !current_password_confirmed?
      @user.assign_attributes(user_params)
      @user.errors.add(:current_password, params.dig(:user, :current_password).blank? ? "is required to change your email address" : "is incorrect")
      set_memberships
      set_two_factor_devices
      return render :show, status: :unprocessable_entity
    end

    previous_email = @user.email_address
    @user.assign_attributes(user_params)
    # A submitted zone (even a blank "Not set") is a choice the member made:
    # browser auto-detect must never overwrite it afterwards.
    @user.time_zone_explicit = true if params[:user]&.key?(:time_zone)
    # A self-chosen email is unverified: Google sign-in will not link a new
    # Google subject to this account by email until an administrator allows it.
    @user.email_self_changed_at = Time.current if email_changing

    if @user.save
      record_account_security_changes(previous_email: previous_email, email_changing: email_changing, password_changing: password_changing)
      redirect_to user_profile_url, notice: update_notice
    else
      set_memberships
      set_two_factor_devices
      render :show, status: :unprocessable_entity
    end
  end

  private
    def set_user
      @user = Current.user
    end

    def set_memberships
      @direct_memberships, @shared_memberships =
        Current.user.memberships.with_ordered_room.partition { |m| m.room.direct? }
    end

    def set_two_factor_devices
      @two_factor_devices = @user.two_factor_remembered_devices.live.recent_first.to_a
    end

    def user_params
      permitted = %i[ name avatar email_address password bio time_zone theme voice_mode push_to_talk_key ]
      # A verified GitHub link owns the login; manual edits are ignored.
      permitted << :github_login unless @user.github_login_verified?
      params.require(:user).permit(*permitted, inbox_preferences: User::InboxPreferences::KEYS).compact
    end

    # Case-only edits are not a change of address: sign-in and Google
    # linking both compare emails case-insensitively.
    def email_change_requested?
      return false unless params[:user].respond_to?(:key?) && params[:user].key?(:email_address)

      !params[:user][:email_address].to_s.strip.casecmp?(@user.email_address.to_s.strip)
    end

    # Accounts without a password (provisioned through Google) have no
    # password to confirm; their change is still recorded as self-made.
    def current_password_confirmed?
      return true if @user.password_digest.blank?

      @user.authenticate(params.dig(:user, :current_password).to_s).present?
    end

    # Password values never reach the log: the row records that a change
    # happened, not what it changed to.
    def record_account_security_changes(previous_email:, email_changing:, password_changing:)
      if email_changing
        AuditLog.record!(action: "user.email.change", actor: @user, target: @user,
          changes: { email_address: AuditLog.pair(previous_email, @user.email_address) })
      end
      if password_changing
        AuditLog.record!(action: "user.password.change", actor: @user, target: @user)
        @user.revoke_two_factor_remembered_devices!
      end
    end

    def update_notice
      params[:user][:avatar] ? "It may take up to 30 minutes to change everywhere." : "✓"
    end
end
