module Slack
  # Connects a member's Slack account through the workspace's internal
  # Slack app (user token only). Started from the admin setup page or the
  # member's personal import page; the callback returns there via an
  # allowlisted path, never an arbitrary URL.
  class OAuthController < ApplicationController
    before_action :require_sudo_mode, only: :start

    # Pages the flow may return to. Validated at start (what is stored)
    # and again at callback (what is used).
    RETURN_PATHS = %w[
      /account/slack_import
      /slack/imports
    ].freeze

    def start
      workspace = SlackWorkspace.current
      unless workspace&.app_configured?
        return redirect_to default_return_path, alert: "Set up the Slack app credentials first."
      end

      raw_state = SecureRandom.hex(16)
      session[:slack_oauth_state] = { "state" => raw_state, "user_id" => Current.user.id }
      session[:slack_oauth_return_to] = validated_return_to(params[:return_to])

      redirect_to Slack::OAuth.authorize_url(
        client_id: workspace.client_id,
        redirect_uri: slack_oauth_callback_url,
        state: state_verifier.generate(raw_state),
        team_id: workspace.team_id
      ), allow_other_host: true
    end

    def callback
      stored = session.delete(:slack_oauth_state) || {}
      return_to = session.delete(:slack_oauth_return_to)
      verified_state = state_verifier.verified(params[:state].to_s)

      unless valid_state?(verified_state, stored)
        return redirect_to validated_return_to(return_to), alert: "Slack connection expired. Try again."
      end

      if params[:error].present?
        return redirect_to validated_return_to(return_to), alert: "Slack connection was not approved."
      end

      workspace = SlackWorkspace.current
      unless workspace&.app_configured?
        return redirect_to validated_return_to(return_to), alert: "The Slack app credentials were removed. Set them up again."
      end

      exchange = Slack::OAuth.exchange_code(
        client_id: workspace.client_id,
        client_secret: workspace.client_secret,
        code: params[:code].to_s,
        redirect_uri: slack_oauth_callback_url
      )
      team_id = exchange.dig("team", "id").to_s
      team_name = exchange.dig("team", "name").to_s
      authed = exchange["authed_user"] || {}
      granted_scopes = authed["scope"].to_s.split(",")

      if workspace.team_id.present? && team_id != workspace.team_id
        return redirect_to validated_return_to(return_to),
          alert: "That Slack account is in a different workspace (#{team_name.presence || team_id}). Connect with an account in #{workspace.team_name.presence || workspace.team_id}."
      end

      if (missing = Slack::OAuth::USER_SCOPES - granted_scopes).any?
        return redirect_to validated_return_to(return_to),
          alert: "Slack did not grant every required permission. Missing: #{missing.join(", ")}. Reconnect and approve them all."
      end

      connection = Current.user.slack_connection || Current.user.build_slack_connection(slack_workspace: workspace)
      connection.assign_attributes(
        slack_workspace: workspace,
        slack_user_id: authed["id"],
        access_token: authed["access_token"],
        scopes: granted_scopes.join(","),
        disconnected_reason: nil
      )
      connection.save!

      # The first admin connection names the workspace being migrated;
      # later grants from any other team are rejected above.
      if workspace.team_id.blank? && Current.user.can_administer?
        domain = Slack::OAuth.team_info(authed["access_token"])&.dig("domain")
        workspace.update!(team_id:, team_name: team_name.presence, team_domain: domain.presence)
      end

      # Only Slack-confirmed, non-secret identifiers reach the log.
      AuditLog.record!(action: "slack.account.connect", actor: Current.user, target: Current.user,
        changes: { slack_user_id: connection.slack_user_id, team_id: team_id })

      redirect_to validated_return_to(return_to), notice: "Slack connected."
    rescue Slack::OAuth::Error
      redirect_to validated_return_to(session.delete(:slack_oauth_return_to)), alert: "Could not connect Slack. Try again."
    end

    private
      def state_verifier
        Rails.application.message_verifier("slack_oauth_state")
      end

      def valid_state?(verified_state, stored)
        stored_state = stored["state"]
        verified_state.is_a?(String) && stored_state.is_a?(String) &&
          verified_state.bytesize == stored_state.bytesize &&
          Rack::Utils.secure_compare(verified_state, stored_state) &&
          stored["user_id"].to_i == Current.user.id
      end

      def validated_return_to(candidate)
        RETURN_PATHS.include?(candidate.to_s) ? candidate.to_s : default_return_path
      end

      def default_return_path
        Current.user.can_administer? ? account_slack_import_path : slack_imports_path
      end
  end
end
