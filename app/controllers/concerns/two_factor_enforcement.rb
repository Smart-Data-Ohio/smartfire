# Enforces two-step sign-in for session-authenticated humans. Runs
# after Authentication, so Current.user and Current.session are set,
# and inside every session restore, so late restores (which run after
# this callback has already passed) are gated too.
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

  # Whitelist: the second-factor flow itself, sign-out, and static
  # endpoints. Health (rails/health) never reaches ApplicationController.
  # Setup exempts only enrollment (show/create): destroy turns 2FA off
  # and must never run on an unverified session, even with the password.
  EXEMPT_CONTROLLER_PATHS = %w[ two_factor/challenges pwa ].freeze
  EXEMPT_SETUP_ACTIONS = %w[ show create ].freeze

  included do
    before_action :require_two_factor_enrollment
  end

  private
    # The Authentication hook: late restores run the same check the
    # callback runs, with the same exemptions, so the setup and
    # challenge pages keep working while every other restored session
    # is gated at restore time.
    def enforce_two_factor_for_restored_session
      require_two_factor_enrollment
    end

    def require_two_factor_enrollment
      return unless two_factor_enforceable?
      return if two_factor_exempt_request?
      # Verified sessions completed the second factor (or came from the
      # test-only sign-in route): nothing to check, and no extra query on
      # the hot path. Only unverified sessions look up the credential.
      return if Current.session.two_factor_verified?

      if Current.user.two_factor_enabled?
        terminate_current_session
        reject_unverified_two_factor_session
      else
        reject_unenrolled_two_factor_user
      end
    end

    def two_factor_enforceable?
      Current.session.present? && authenticated_by.session? &&
        Current.user&.requires_two_factor?
    end

    def two_factor_exempt_request?
      EXEMPT_CONTROLLER_PATHS.include?(controller_path) ||
        (controller_path == "two_factor/setups" && EXEMPT_SETUP_ACTIONS.include?(action_name)) ||
        (controller_path == "sessions" && action_name == "destroy")
    end

    # Redirects HTML and Turbo Stream requests to setup (Turbo follows
    # the redirect, so stream requests land on setup instead of bypassing
    # it); other formats get a bare rejection.
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
