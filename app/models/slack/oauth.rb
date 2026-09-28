require "net/http"

module Slack
  # OAuth v2 against the workspace's internal Slack app. User tokens only
  # (xoxp-): a user token reads every public channel plus the private
  # channels, DMs and group DMs its member is in, so no bot scopes, bot
  # user, or events are ever requested.
  #
  # Never logs tokens or secrets; errors carry only HTTP statuses and the
  # Slack error code.
  class OAuth
    AUTHORIZE_URL = "https://slack.com/oauth/v2/authorize"
    ACCESS_TOKEN_URL = "https://slack.com/api/oauth.v2.access"
    REVOKE_URL = "https://slack.com/api/auth.revoke"
    TEAM_INFO_URL = "https://slack.com/api/team.info"
    TIMEOUT = 10

    # Kept in sync with the manifest (Slack::AppManifest) and the grants
    # the callback requires: every scope here must be present on the
    # token or the connection is rejected.
    USER_SCOPES = %w[
      channels:history channels:read groups:history groups:read im:history
      im:read mpim:history mpim:read users:read users:read.email team:read
    ].freeze

    class Error < StandardError; end

    class << self
      # No `scope` (bot scopes) param: user_scope alone keeps the grant
      # a pure user-token install. `team` pins the chooser once the
      # workspace's team id is known.
      def authorize_url(client_id:, redirect_uri:, state:, team_id: nil)
        uri = URI(AUTHORIZE_URL)
        params = { client_id:, redirect_uri:, state:, user_scope: USER_SCOPES.join(",") }
        params[:team] = team_id if team_id.present?
        uri.query = URI.encode_www_form(params)
        uri.to_s
      end

      # Exchanges an authorization code for the install. Returns the
      # parsed oauth.v2.access body; the user token lives at
      # authed_user.access_token with its granted scopes at
      # authed_user.scope, and the team at team.id / team.name.
      #
      # Raises Error carrying only the Slack error code (invalid_code,
      # bad_client_secret, ...) or the transport failure class.
      #
      # Request and response shapes per
      # https://docs.slack.dev/reference/methods/oauth.v2.access
      def exchange_code(client_id:, client_secret:, code:, redirect_uri:)
        uri = URI(ACCESS_TOKEN_URL)
        response = Net::HTTP.start(uri.host, uri.port, use_ssl: true,
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          http.post(uri.request_uri,
            URI.encode_www_form(client_id:, client_secret:, code:, redirect_uri:),
            { "Content-Type" => "application/x-www-form-urlencoded", "Accept" => "application/json" })
        end

        body = JSON.parse(response.body.to_s)
        if response.is_a?(Net::HTTPSuccess) && body["ok"] && body.dig("authed_user", "access_token").present?
          body
        else
          raise Error, "Slack rejected the OAuth grant (#{body["error"].presence || response.code})"
        end
      rescue Error
        raise
      rescue StandardError => error
        Rails.logger.warn "Slack::OAuth code exchange failed: #{error.class}"
        raise Error, "Could not reach Slack (#{error.class.name.demodulize.titleize})"
      end

      # Best-effort revocation of one user token, for disconnect. Only a
      # revoked:true answer counts; anything else (already dead token,
      # transport failure) still lets the local disconnect proceed.
      # Never raises. See https://docs.slack.dev/reference/methods/auth.revoke
      def revoke(token)
        uri = URI(REVOKE_URL)
        response = Net::HTTP.start(uri.host, uri.port, use_ssl: true,
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          http.post(uri.request_uri, URI.encode_www_form(token:),
            { "Content-Type" => "application/x-www-form-urlencoded", "Accept" => "application/json" })
        end

        JSON.parse(response.body.to_s)["revoked"] == true
      rescue StandardError => error
        Rails.logger.warn "Slack::OAuth revoke failed: #{error.class}"
        false
      end

      # Best-effort team.info lookup for the workspace domain (the OAuth
      # response carries the team id and name, not the domain). Returns
      # the team hash or nil. Never raises.
      def team_info(token)
        uri = URI(TEAM_INFO_URL)
        response = Net::HTTP.start(uri.host, uri.port, use_ssl: true,
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          request = Net::HTTP::Get.new(uri.request_uri, {
            "Accept" => "application/json",
            "Authorization" => "Bearer #{token}"
          })
          http.request(request)
        end

        body = JSON.parse(response.body.to_s)
        body["ok"] ? body["team"] : nil
      rescue StandardError => error
        Rails.logger.warn "Slack::OAuth team.info failed: #{error.class}"
        nil
      end
    end
  end
end
