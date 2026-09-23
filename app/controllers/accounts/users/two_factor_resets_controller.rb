# Administrator recovery for a member who lost their authenticator:
# destroys the credential, backup codes, and remembered devices, and
# signs the member out everywhere, so they re-enroll at next sign-in.
# Administrators cannot reset their own 2FA this way.
class Accounts::Users::TwoFactorResetsController < ApplicationController
  before_action :ensure_can_administer
  before_action :set_user

  def create
    if @user == Current.user
      return redirect_to edit_account_url, alert: "Reset someone else's two-step sign-in from here. To change your own, use Disable on your profile."
    end
    unless @user.two_factor_enabled?
      return redirect_to edit_account_url, alert: "#{@user.name} doesn't have two-step sign-in enabled."
    end

    @user.reset_two_factor!
    @user.sessions.destroy_all
    disconnect_remote_connections(@user)
    AuditLog.record!(action: "two_factor.reset", target: @user)
    redirect_to edit_account_url, notice: "Two-step sign-in reset for #{@user.name}. They will set it up again at next sign-in."
  end

  private
    def set_user
      @user = User.active.without_bots.find(params[:user_id])
    end
end
