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
      credential = @pending_user.two_factor_credential

      if credential&.locked_out?
        render_locked_out credential
      elsif verified_as = verify_challenge_code
        credential.register_challenge_success!
        method = two_factor_pending_method
        clear_two_factor_pending!
        start_new_session_for @pending_user, two_factor_verified: true
        remember_two_factor_device!(@pending_user) if params[:remember_device] == "1"
        AuditLog.record!(action: "session.sign_in.success", actor: @pending_user,
          changes: { method: method, two_factor: verified_as })
        redirect_to post_authenticating_url
      else
        register_failure_and_render credential
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

      def register_failure_and_render(credential)
        locked = credential&.register_challenge_failure! == :locked
        AuditLog.record!(action: "sign_in.two_factor.failure", actor: @pending_user,
          changes: { method: two_factor_pending_method })

        if locked
          credential.reload
          AuditLog.record!(action: "sign_in.two_factor.lockout", actor: @pending_user,
            changes: { method: two_factor_pending_method })
          notify_two_factor_lockout!
          render_locked_out credential
        else
          flash.now[:alert] = "That code didn't work. Check your authenticator app or try a backup code."
          render :show, status: :unprocessable_entity
        end
      end

      # Locked attempts (even correct ones) change nothing: no code is
      # spent, no backup code burns, and the failure run does not grow.
      def render_locked_out(credential)
        minutes = ((credential.locked_until - Time.current) / 60.0).ceil
        flash.now[:alert] = "Too many wrong codes. Try again in #{minutes} #{'minute'.pluralize(minutes)}."
        render :show, status: :too_many_requests
      end

      def notify_two_factor_lockout!
        # One row per recipient + source: a repeat lockout resurfaces the
        # same item (back at the top, unread again) instead of stacking.
        item = ActivityItem.find_or_initialize_by(user: @pending_user,
          source: @pending_user.two_factor_credential, event_type: "two_factor_lockout")
        if item.persisted?
          item.mark_unread!
          item.touch
        else
          item.save!
        end

        if TwoFactorMailer.mail_configured?
          TwoFactorMailer.lockout_notice(@pending_user).deliver_later
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
