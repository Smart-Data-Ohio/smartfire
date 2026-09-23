# Enforces two-step sign-in for session-authenticated humans. Runs
# after Authentication, so Current.user and Current.session are set.
#
# - A human without 2FA lands on the setup page before anything else.
# - A human with 2FA but a session that never completed the second
#   factor (stale pre-2FA sessions, or a bug) is signed out and sent
#   back to sign in.
# - Bots and agent tokens authenticate with keys and are exempt.
# - Non-HTML requests (JSON, Turbo Streams) are rejected, never served
#   and never silently redirected into a bypass.
module TwoFactorEnforcement
  extend ActiveSupport::Concern

  # Whitelist: the two-factor flow itself, sign-out, and static
  # endpoints. Health (rails/health) never reaches ApplicationController.
  EXEMPT_CONTROLLER_PATHS = %w[ two_factor/setups two_factor/challenges pwa ].freeze

  included do
    before_action :require_two_factor_enrollment
  end

  private
    def require_two_factor_enrollment
      return unless two_factor_enforceable?
      return if two_factor_exempt_request?

      if Current.user.two_factor_enabled?
        unless Current.session.two_factor_verified?
          terminate_current_session
          reject_unverified_two_factor_session
        end
      elsif !Current.session.two_factor_verified?
        reject_unenrolled_two_factor_user
      end
    end

    def two_factor_enforceable?
      Current.session.present? && authenticated_by.session? &&
        Current.user&.requires_two_factor?
    end

    def two_factor_exempt_request?
      EXEMPT_CONTROLLER_PATHS.include?(controller_path) ||
        (controller_path == "sessions" && action_name == "destroy")
    end

    def reject_unenrolled_two_factor_user
      if request.format.html?
        session[:return_to_after_authenticating] = request.url if request.get?
        redirect_to two_factor_setup_url
      else
        head :forbidden
      end
    end

    def reject_unverified_two_factor_session
      if request.format.html?
        redirect_to new_session_url, alert: "Sign in again to verify two-step sign-in."
      else
        head :unauthorized
      end
    end
end
