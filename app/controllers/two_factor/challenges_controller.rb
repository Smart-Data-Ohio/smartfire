module TwoFactor
  # The second step after password, Google, or transfer sign-in. The
  # pending user has no session yet (Current.user stays nil); only a
  # valid TOTP or backup code creates one. Accepts TOTP codes and
  # backup codes in the same field.
  class ChallengesController < ApplicationController
    allow_unauthenticated_access only: %i[ show create ]

    rate_limit to: 10, within: 3.minutes, only: :create, with: -> { render_rate_limited }
    rate_limit to: 10, within: 15.minutes, only: :create, name: "per-user",
      by: -> { session[TWO_FACTOR_PENDING_USER_KEY] }, with: -> { render_rate_limited }

    before_action :restore_authentication, only: %i[ show create ]
    before_action :set_pending_user, only: %i[ show create ]

    def show
      no_store_response!
    end

    def create
      no_store_response!

      verified_as = verify_challenge_code
      if verified_as
        method = two_factor_pending_method
        remember_two_factor_device!(@pending_user) if params[:remember_device] == "1"
        clear_two_factor_pending!
        start_new_session_for @pending_user, two_factor_verified: true
        AuditLog.record!(action: "session.sign_in.success", actor: @pending_user,
          changes: { method: method, two_factor: verified_as })
        redirect_to post_authenticating_url
      else
        AuditLog.record!(action: "sign_in.two_factor.failure", actor: @pending_user,
          changes: { method: two_factor_pending_method })
        flash.now[:alert] = "That code didn't work. Check your authenticator app or try a backup code."
        render :show, status: :unprocessable_entity
      end
    end

    private
      def set_pending_user
        return redirect_to root_url if signed_in?

        @pending_user = two_factor_pending_user
        if @pending_user.nil?
          clear_two_factor_pending!
          redirect_to new_session_url
        elsif !@pending_user.two_factor_enabled?
          # Reset or disabled after the first factor: fall back to it.
          clear_two_factor_pending!
          redirect_to new_session_url
        end
      end

      def verify_challenge_code
        credential = @pending_user.two_factor_credential
        return nil unless credential&.enabled?

        code = params[:code].to_s
        if credential.verify_code(code)
          "totp"
        elsif TwoFactorBackupCode.consume!(credential, code)
          "backup_code"
        end
      end

      def render_rate_limited
        no_store_response!
        @pending_user = two_factor_pending_user
        AuditLog.record!(action: "sign_in.two_factor.failure", actor: @pending_user,
          changes: { method: two_factor_pending_method, rate_limited: true })
        flash.now[:alert] = "Too many attempts. Try again in a few minutes."
        render :show, status: :too_many_requests
      end
  end
end
