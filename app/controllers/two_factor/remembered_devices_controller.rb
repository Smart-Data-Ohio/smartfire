module TwoFactor
  # Revokes remembered devices from the profile: one device, or every
  # device at once. The cookie becomes useless once its server-side row
  # is gone. Both need a fresh TOTP code, the password, or a completed
  # Google re-auth in the same request.
  class RememberedDevicesController < ApplicationController
    include TwoFactorReauthentication

    rate_limit to: 10, within: 3.minutes, only: %i[ destroy destroy_all ], with: -> { render_rate_limited }
    rate_limit to: 10, within: 15.minutes, only: %i[ destroy destroy_all ], name: "per-user",
      by: -> { Current.user&.id }, with: -> { render_rate_limited }

    def destroy
      return refuse_without_reauthentication(Current.user) unless reauthenticated?(Current.user)

      Current.user.two_factor_remembered_devices.find_by(id: params[:id])&.destroy!
      redirect_to user_profile_url, notice: "Device forgotten. It will ask for a code at next sign-in."
    end

    def destroy_all
      return refuse_without_reauthentication(Current.user) unless reauthenticated?(Current.user)

      Current.user.revoke_two_factor_remembered_devices!
      AuditLog.record!(action: "two_factor.devices.revoke_all", target: Current.user)
      redirect_to user_profile_url, notice: "All devices forgotten. Every browser will ask for a code at next sign-in."
    end

    private
      def render_rate_limited
        redirect_to user_profile_url, alert: "Too many attempts. Try again in a few minutes."
      end
  end
end
