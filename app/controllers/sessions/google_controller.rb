module Sessions
  # Unauthenticated Workspace "Sign in with Google". Separate from the
  # Calendar/Drive connection flow (Google::ConnectionsController),
  # which requires login and stays opt-in: this flow requests only
  # "openid email profile" and persists no OAuth tokens.
  class GoogleController < ApplicationController
    include GoogleSignInFlow

    require_unauthenticated_access only: :create
    # The callback also finishes a signed-in member's "Link Google sign-in"
    # flow, so it restores the session instead of refusing signed-in users;
    # a sign-in flow still sends a signed-in browser home.
    allow_unauthenticated_access only: :callback
    before_action :restore_session_for_callback, only: :callback

    before_action :ensure_configured
    before_action :ensure_workspace_ready

    # Starts the flow: stash a browser-bound one-use state, nonce, and
    # PKCE verifier in the session, then send the browser to Google.
    def create
      redirect_to_google_sign_in(purpose: "sign_in")
    end

    # Handles Google's redirect back: consume the one-use flow, trade
    # the code for an id_token, verify it, and sign the user in. Every
    # failure lands back on the login page with the password form
    # intact -- Google sign-in never restricts global access.
    def callback
      flow = session.delete(GoogleSignInFlow::FLOW_SESSION_KEY)
      verified_state = google_sign_in_state_verifier.verified(params[:state].to_s)

      if valid_flow?(flow, verified_state) && flow["purpose"] == "link"
        return finish_link(flow)
      end
      if valid_flow?(flow, verified_state) && flow["purpose"] == "reauth"
        return finish_reauth(flow)
      end
      return redirect_to root_url if signed_in?

      unless valid_flow?(flow, verified_state)
        return redirect_to new_session_url, alert: "Google sign-in expired. Try again or sign in with email and password."
      end

      if params[:error].present?
        return redirect_to new_session_url, alert: "Google sign-in was cancelled. Try again or sign in with email and password."
      end

      if params[:code].blank?
        return redirect_to new_session_url, alert: "Google sign-in failed. Try again or sign in with email and password."
      end

      id_token = Google::SignIn.exchange_code(
        code: params[:code].to_s,
        redirect_uri: session_google_callback_url,
        verifier: flow["verifier"]
      )
      claims = Google::SignIn::IdTokenVerifier.verify!(id_token, nonce: flow["nonce"])

      # SQLite transactions acquire the configured immediate write lock.
      # Keep eligibility, identity linking, and session insertion together:
      # deactivation/ban must either win before this check or revoke the new
      # session afterwards. All network verification stays outside the lock.
      # Enrolled users get no session here: begin_session_for leaves them
      # pending for the challenge instead.
      User.transaction do
        user = Google::SignIn::AccountLinker.resolve!(claims)
        if user.previously_new_record?
          AuditLog.record!(action: "user.create", actor: user, target: user, changes: { method: "google" })
        elsif user.google_identity&.previously_new_record?
          AuditLog.record!(action: "google.sign_in.link", actor: user, target: user,
            changes: { email: claims["email"] })
        end
        begin_session_for user, method: "google", return_url: safe_post_authenticating_url
      end
    rescue Google::SignIn::Unavailable
      redirect_to new_session_url, alert: "Google sign-in is unavailable right now. Try again or sign in with email and password."
    rescue Google::SignIn::Rejected => error
      Rails.logger.warn "Google sign-in rejected: #{error.reason}"
      AuditLog.record_sign_in_failure!(email: "", method: "google")
      redirect_to new_session_url, alert: rejection_alert(error.reason)
    end

    private
      # A separate name: re-declaring restore_authentication here would
      # replace the create action's copy from require_unauthenticated_access.
      def restore_session_for_callback
        restore_authentication
      end

      def ensure_configured
        head :not_found unless Google::SignIn.configured?
      end

      # Google sign-in can only join an existing workspace; it must
      # never bypass first-run setup by provisioning its first users.
      def ensure_workspace_ready
        redirect_to first_run_url if Account.none? || User.none?
      end

      # Links the verified Google subject to the member who started the
      # flow, who must still be the signed-in user. Same verification as
      # sign-in: PKCE, nonce, audience, and the hd domain allowlist.
      def finish_link(flow)
        unless signed_in? && Current.user.id == flow["user_id"]
          return redirect_to(signed_in? ? user_profile_url : new_session_url, alert: "Google linking expired. Try again.")
        end

        if params[:error].present? || params[:code].blank?
          return redirect_to user_profile_url, alert: "Google linking was cancelled."
        end

        id_token = Google::SignIn.exchange_code(
          code: params[:code].to_s, redirect_uri: session_google_callback_url, verifier: flow["verifier"]
        )
        claims = Google::SignIn::IdTokenVerifier.verify!(id_token, nonce: flow["nonce"])
        User.transaction do
          Google::SignIn::AccountLinker.link_to_user!(claims, Current.user)
          AuditLog.record!(action: "google.sign_in.link", actor: Current.user, target: Current.user,
            changes: { email: claims["email"] })
        end

        redirect_to user_profile_url, notice: "Google sign-in linked to #{claims["email"]}."
      rescue Google::SignIn::Unavailable
        redirect_to user_profile_url, alert: "Google is unavailable right now. Try again."
      rescue Google::SignIn::Rejected => error
        Rails.logger.warn "Google link rejected: #{error.reason}"
        redirect_to user_profile_url, alert: link_rejection_alert(error.reason)
      end

      # Finishes a signed-in member's "Confirm with Google" step-up: the
      # verified subject must be the SAME Google account already linked to
      # them, or any Google account could arm someone else's sensitive
      # actions. Success arms a single-use re-authentication (see
      # TwoFactorReauthentication) and lands back on the profile.
      def finish_reauth(flow)
        unless signed_in? && Current.user.id == flow["user_id"]
          return redirect_to(signed_in? ? user_profile_url : new_session_url, alert: "Google confirmation expired. Try again.")
        end

        if params[:error].present? || params[:code].blank?
          return redirect_to user_profile_url, alert: "Google confirmation was cancelled."
        end

        id_token = Google::SignIn.exchange_code(
          code: params[:code].to_s, redirect_uri: session_google_callback_url, verifier: flow["verifier"]
        )
        claims = Google::SignIn::IdTokenVerifier.verify!(id_token, nonce: flow["nonce"],
          max_auth_age: Google::SignIn::REAUTH_MAX_AUTH_AGE)

        unless claims["sub"].to_s.present? && claims["sub"].to_s == Current.user.google_identity&.subject
          return redirect_to user_profile_url, alert: "That Google account is not linked here. Confirm with the Google account you sign in with."
        end

        session[TwoFactorReauthentication::REAUTH_SESSION_KEY] = Time.current.to_i
        AuditLog.record!(action: "two_factor.reauthenticate", actor: Current.user, target: Current.user)

        redirect_to user_profile_url, notice: "Confirmed with Google. Continue with what you were doing."
      rescue Google::SignIn::Unavailable
        redirect_to user_profile_url, alert: "Google is unavailable right now. Try again."
      rescue Google::SignIn::Rejected => error
        Rails.logger.warn "Google re-auth rejected: #{error.reason}"
        redirect_to user_profile_url, alert: "Google confirmation failed. Try again."
      end

      def link_rejection_alert(reason)
        case reason
        when :wrong_domain then "Only #{domain_list} Google accounts can be linked."
        when :subject_taken then "That Google account already signs in as another member."
        when :already_linked then "Your account is already linked to a Google account. Ask an administrator to unlink it first."
        else "Google linking failed. Try again."
        end
      end

      def valid_flow?(flow, verified_state)
        flow.is_a?(Hash) &&
          flow["state"].is_a?(String) && flow["nonce"].is_a?(String) &&
          flow["verifier"].is_a?(String) && flow["exp"].is_a?(Integer) &&
          flow["exp"] > Time.current.to_i &&
          verified_state.is_a?(String) &&
          verified_state.bytesize == flow["state"].bytesize &&
          Rack::Utils.secure_compare(verified_state, flow["state"])
      end

      def rejection_alert(reason)
        case reason
        when :wrong_domain
          "Google sign-in is only available for #{domain_list}. Other email addresses can sign in with email and password."
        when :ambiguous, :subject_mismatch
          "Google sign-in could not pick your account. Contact your administrator or sign in with email and password."
        when :admin_link_required
          "An administrator must link this account to Google before you can sign in with Google. Contact your administrator or sign in with email and password."
        when :deactivated, :banned
          "This account is no longer active. Contact your administrator or sign in with email and password."
        else
          "Google sign-in failed. Try again or sign in with email and password."
        end
      end

      def domain_list
        Google::SignIn.allowed_domains.map { |domain| "@#{domain}" }.to_sentence
      end

      def safe_post_authenticating_url
        Google::SignIn.safe_return_path(
          session.delete(:return_to_after_authenticating), host: request.host
        ) || root_url
      end
  end
end
