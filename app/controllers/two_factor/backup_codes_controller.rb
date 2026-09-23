module TwoFactor
  # Regenerates the signed-in user's backup codes from the profile,
  # invalidating the old set, and shows the new set once. Needs a fresh
  # TOTP code, the password, or a completed Google re-auth in the same
  # request: the new set would otherwise let anyone holding the session
  # sign in again later.
  class BackupCodesController < ApplicationController
    include TwoFactorReauthentication

    rate_limit to: 10, within: 3.minutes, only: :create, with: -> { render_rate_limited }
    rate_limit to: 10, within: 15.minutes, only: :create, name: "per-user",
      by: -> { Current.user&.id }, with: -> { render_rate_limited }

    def create
      no_store_response!
      credential = Current.user.two_factor_credential
      return redirect_to two_factor_setup_url unless credential&.enabled?
      return refuse_without_reauthentication(Current.user) unless reauthenticated?(Current.user)

      @backup_codes = TwoFactorBackupCode.regenerate_set!(credential)
      AuditLog.record!(action: "two_factor.backup_codes.regenerate", target: Current.user)
      @continue_url = user_profile_url
      render :show
    end

    private
      def render_rate_limited
        redirect_to user_profile_url, alert: "Too many attempts. Try again in a few minutes."
      end
  end
end
