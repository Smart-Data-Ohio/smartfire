module Github
  # Connects a member's GitHub identity through the workspace GitHub App
  # (short-lived user-to-server tokens with refresh). Disabled cleanly
  # while the App is unconfigured: the routes answer 404 and the profile
  # offers only the PAT form. Disconnect revokes the App grant remotely
  # (best effort) before deleting the row.
  class AppConnectionsController < ApplicationController
    before_action :ensure_configured

    def connect
      raw_state = SecureRandom.hex(16)
      session[:github_app_oauth_state] = raw_state
      redirect_to Github::App.authorize_url(
        redirect_uri: github_app_callback_url,
        state: state_verifier.generate(raw_state)
      ), allow_other_host: true
    end

    def callback
      stored_state = session.delete(:github_app_oauth_state)
      verified_state = state_verifier.verified(params[:state].to_s)

      unless valid_state?(verified_state, stored_state)
        return redirect_to user_profile_path, alert: "GitHub connection expired. Try again."
      end

      if params[:error].present?
        return redirect_to user_profile_path, alert: "GitHub connection was not approved."
      end

      tokens = Github::App.exchange_code(code: params[:code].to_s, redirect_uri: github_app_callback_url)
      login = WriteClient.authenticated_login(tokens["access_token"])

      account = Current.user.github_connected_account || Current.user.build_github_connected_account
      old_app_token = account.app_token_for_revoke
      account.assign_attributes(
        github_login: login,
        access_token: tokens["access_token"],
        refresh_token: tokens["refresh_token"],
        token_expires_at: (Time.current + tokens["expires_in"].to_i.seconds if tokens["expires_in"]),
        token_source: "app",
        disconnected_reason: nil,
        last_error: nil
      )
      account.save!
      account.touch
      # A replaced App token is revoked remotely (best effort, after the
      # save) so no orphaned token survives the relink. Single-token
      # revocation only: deleting the grant would take the new token
      # with it.
      if old_app_token.present? && old_app_token != tokens["access_token"]
        Github::App.revoke_token(old_app_token)
      end

      redirect_to user_profile_path, notice: "GitHub connected as #{login}."
    rescue Github::App::Unauthorized, WriteClient::Unauthorized
      redirect_to user_profile_path, alert: "GitHub rejected the connection. Try again."
    rescue Github::App::Error, WriteClient::Error
      redirect_to user_profile_path, alert: "Could not reach GitHub. Try again."
    end

    private
      def ensure_configured
        head :not_found unless Github::App.configured?
      end

      def state_verifier
        Rails.application.message_verifier("github_app_oauth_state")
      end

      def valid_state?(verified_state, stored_state)
        verified_state.is_a?(String) && stored_state.is_a?(String) &&
          verified_state.bytesize == stored_state.bytesize &&
          Rack::Utils.secure_compare(verified_state, stored_state)
      end
  end
end
