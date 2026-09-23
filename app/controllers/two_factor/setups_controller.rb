module TwoFactor
  # Mandatory enrollment: the enforcement redirect lands every signed-in
  # human without 2FA here. The page shows a QR code plus the manual
  # key; confirming with a valid code enables 2FA and shows the backup
  # codes once. Destroy disables 2FA, which immediately re-triggers the
  # enrollment redirect.
  class SetupsController < ApplicationController
    include TwoFactorReauthentication

    rate_limit to: 10, within: 3.minutes, only: :create, with: -> { render_rate_limited }
    rate_limit to: 10, within: 15.minutes, only: :create, name: "per-user",
      by: -> { Current.user&.id }, with: -> { render_rate_limited }
    rate_limit to: 10, within: 3.minutes, only: :destroy, with: -> { render_destroy_rate_limited }
    rate_limit to: 10, within: 15.minutes, only: :destroy, name: "per-user-destroy",
      by: -> { Current.user&.id }, with: -> { render_destroy_rate_limited }

    before_action :ensure_human_user

    def show
      no_store_response!
      return redirect_to user_profile_url if Current.user.two_factor_enabled?

      # A fresh secret on every visit: a secret displayed here is only
      # ever confirmable from this session (see TwoFactorSetupSecret).
      issue_setup_presentation
    end

    def create
      no_store_response!
      return redirect_to user_profile_url if Current.user.two_factor_enabled?

      setup_secret = TwoFactorSetupSecret.valid_for(Current.session)
      credential = enrollable_credential
      if setup_secret && credential.confirm_with_setup_secret!(setup_secret, params[:code].to_s)
        @backup_codes = TwoFactorBackupCode.regenerate_set!(credential)
        Current.session.mark_two_factor_verified!
        AuditLog.record!(action: "two_factor.enable", target: Current.user)
        @continue_url = post_authenticating_url
        render "two_factor/backup_codes/show"
      else
        # Retry against the same live secret (re-scanning on every typo
        # would be unusable); a missing or expired secret rotates, and
        # the old code stops working.
        reuse_setup_presentation
        flash.now[:alert] = "That code didn't work. Check your authenticator app and try again."
        render :show, status: :unprocessable_entity
      end
    end

    # Disabling needs a fresh TOTP code, the password, or a completed
    # Google re-auth in the same request: anyone holding the session
    # could otherwise disable and re-enroll on their own authenticator.
    def destroy
      if Current.user.two_factor_enabled?
        return refuse_without_reauthentication(Current.user) unless reauthenticated?(Current.user)

        Current.user.reset_two_factor!
        Current.session.clear_two_factor_verified!
        # Every session re-enrolls, not just this one: a live session must
        # never keep browsing after its second factor is gone.
        Current.user.sessions.update_all(two_factor_verified_at: nil)
        cookies.delete(TWO_FACTOR_REMEMBER_COOKIE)
        AuditLog.record!(action: "two_factor.disable", target: Current.user)
        redirect_to two_factor_setup_url, notice: "Two-step sign-in is off. Set it up again to keep signing in."
      else
        redirect_to two_factor_setup_url
      end
    end

    private
      def ensure_human_user
        redirect_to root_url unless Current.user.requires_two_factor?
      end

      def enrollable_credential
        Current.user.two_factor_credential ||
          TwoFactorCredential.create_or_find_by!(user: Current.user) do |credential|
            credential.secret = TwoFactorCredential.generate_secret
          end
      end

      # The page renders a transient credential carrying the session's
      # pending secret: the stored row is only written at confirm time.
      def issue_setup_presentation
        present_setup_secret TwoFactorSetupSecret.issue_for!(Current.session)
      end

      def reuse_setup_presentation
        present_setup_secret(
          TwoFactorSetupSecret.valid_for(Current.session) ||
            TwoFactorSetupSecret.issue_for!(Current.session)
        )
      end

      def present_setup_secret(setup_secret)
        @credential = TwoFactorCredential.new(user: Current.user, secret: setup_secret.secret)
        @provisioning_uri = @credential.provisioning_uri
      end

      def render_rate_limited
        no_store_response!
        return redirect_to user_profile_url if Current.user&.two_factor_enabled?

        reuse_setup_presentation
        flash.now[:alert] = "Too many attempts. Try again in a few minutes."
        render :show, status: :too_many_requests
      end

      def render_destroy_rate_limited
        redirect_to user_profile_url, alert: "Too many attempts. Try again in a few minutes."
      end
  end
end
