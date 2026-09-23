require "net/http"
require "base64"

module Github
  # OAuth against the workspace GitHub App for per-member user-to-server
  # tokens. App tokens are short-lived with refresh; they replace pasted
  # personal access tokens for people, while the PAT path stays as a
  # fallback during migration. Disabled cleanly while unconfigured: the
  # connect routes answer 404 and the profile offers only the PAT form.
  #
  # Never logs tokens or secrets; error messages carry only HTTP statuses.
  class App
    AUTHORIZE_HOST = "github.com"
    TOKEN_HOST = "github.com"
    API_HOST = "api.github.com"
    TIMEOUT = 10

    class Error < StandardError; end
    class Unauthorized < Error; end

    class << self
      def configured?
        client_id.present? && client_secret.present?
      end

      def client_id
        ENV["GITHUB_APP_CLIENT_ID"].presence
      end

      def client_secret
        ENV["GITHUB_APP_CLIENT_SECRET"].presence
      end

      def authorize_url(redirect_uri:, state:)
        uri = URI::HTTPS.build(host: AUTHORIZE_HOST, path: "/login/oauth/authorize")
        uri.query = URI.encode_www_form(client_id:, redirect_uri:, state:, scope: "")
        uri.to_s
      end

      # Exchange an authorization code for a short-lived user token plus
      # its refresh token. Returns the parsed token response.
      def exchange_code(code:, redirect_uri:)
        post_oauth_form(
          client_id:, client_secret:, code:, redirect_uri:
        )
      end

      # Refresh a short-lived user token. Returns the parsed token
      # response (a new refresh token included: GitHub rotates it).
      def refresh_access_token(refresh_token:)
        post_oauth_form(
          client_id:, client_secret:,
          grant_type: "refresh_token", refresh_token:
        )
      end

      # Best-effort revocation of a single App user token, for relink:
      # the replaced token dies while the replacement grant's tokens
      # survive. Returns true when GitHub accepted (or no longer knows)
      # the token, false otherwise. Never raises: relink must not fail
      # because revocation did.
      #
      # DELETE /applications/{client_id}/token revokes one token; the
      # /grant sibling deletes the whole authorization, including every
      # other token issued for it, so it must never run on relink:
      # https://docs.github.com/en/rest/apps/oauth-applications?apiVersion=2022-11-28
      def revoke_token(token)
        delete_application_path("token", token)
      end

      # Best-effort revocation of the whole App authorization, for a full
      # disconnect only: GitHub deletes every token issued for the grant
      # and drops the authorization screen entry. Same best-effort
      # contract as revoke_token. Never raises.
      #
      # https://docs.github.com/en/rest/apps/oauth-applications?apiVersion=2022-11-28
      def revoke_grant(token)
        delete_application_path("grant", token)
      end

      private
        def delete_application_path(segment, token)
          uri = URI::HTTPS.build(host: API_HOST, path: "/applications/#{client_id}/#{segment}")
          response = Net::HTTP.start(uri.host, uri.port, use_ssl: true,
              open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
            request = Net::HTTP::Delete.new(uri.request_uri, {
              "Content-Type" => "application/json",
              "Accept" => "application/vnd.github+json",
              "Authorization" => "Basic #{Base64.strict_encode64("#{client_id}:#{client_secret}")}"
            })
            request.body = { access_token: token }.to_json
            http.request(request)
          end
          response.is_a?(Net::HTTPNoContent) || response.code == "404"
        rescue StandardError => error
          Rails.logger.warn "Github::App revoke failed: #{error.class}"
          false
        end

        def post_oauth_form(params)
          uri = URI::HTTPS.build(host: TOKEN_HOST, path: "/login/oauth/access_token")
          response = Net::HTTP.start(uri.host, uri.port, use_ssl: true,
              open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
            http.post(uri.request_uri, URI.encode_www_form(params), {
              "Content-Type" => "application/x-www-form-urlencoded",
              "Accept" => "application/json"
            })
          end

          body = JSON.parse(response.body.to_s)
          if response.is_a?(Net::HTTPSuccess) && body["access_token"].present?
            body
          elsif response.code == "401" || body["error"] == "invalid_grant"
            raise Unauthorized, "GitHub rejected the GitHub App grant (401)"
          else
            raise Error, "GitHub App token request failed (#{response.code})"
          end
        rescue Error
          raise
        rescue StandardError => error
          Rails.logger.warn "Github::App token request failed: #{error.class}"
          raise Error, "Could not reach GitHub (#{error.class.name.demodulize.titleize})"
        end
    end
  end
end
