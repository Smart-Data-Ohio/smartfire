require "net/http"
require "base64"
require "openssl"

module Google
  # Minimal Google OAuth + Calendar client over Net::HTTP. All network access
  # for Google Calendar publishing goes through here so tests can stub it
  # with WebMock. Never logs tokens; error messages carry only HTTP statuses
  # and Google's error codes.
  class Client
    AUTHORIZE_HOST = "accounts.google.com"
    TOKEN_HOST = "oauth2.googleapis.com"
    API_HOST = "www.googleapis.com"
    TIMEOUT = 10
    CALENDAR_SCOPE = "https://www.googleapis.com/auth/calendar.events"
    SCOPE = "openid email #{CALENDAR_SCOPE}"
    # Per-file access only: the grant covers files the member opens with
    # Smartfire through the Picker, never the whole Drive. Pasted links to
    # other files answer 403/404 from Google and render as plain chips.
    DRIVE_SCOPE = "https://www.googleapis.com/auth/drive.file"
    ID_TOKEN_ISSUERS = %w[ https://accounts.google.com accounts.google.com ].freeze

    # In-memory credentials for post-disconnect cleanup, when the account
    # row (and its tokens) is already gone. Quacks like the account bits
    # the client touches: refreshes update memory instead of the database,
    # and there is no row left to mark disconnected.
    class SnapshotCredentials
      attr_reader :refresh_token
      attr_accessor :access_token, :access_token_expires_at

      def initialize(access_token:, refresh_token:, access_token_expires_at:)
        @access_token = access_token
        @refresh_token = refresh_token
        @access_token_expires_at = access_token_expires_at
      end

      def access_token_expired?
        access_token.blank? || access_token_expires_at.blank? || access_token_expires_at <= Time.current
      end

      def update!(access_token:, access_token_expires_at:)
        self.access_token = access_token
        self.access_token_expires_at = access_token_expires_at
      end

      def mark_disconnected!(_reason)
        nil
      end
    end

    class Error < StandardError; end
    class Unavailable < Error; end
    class RateLimited < Unavailable; end
    class Unauthorized < Error; end
    class Forbidden < Error; end
    class NotFound < Error; end
    class Conflict < Error; end

    # Timeouts, connection failures, and malformed bodies surface as
    # Unavailable so callers and last_error see one shape. Messages carry
    # only the error class, never response bytes that could hold tokens.
    # SystemCallError covers every Errno::* connection failure; IOError
    # covers EOFError from a dropped connection.
    TRANSPORT_ERRORS = [
      Net::OpenTimeout, Net::ReadTimeout, Net::WriteTimeout,
      Net::HTTPBadResponse, SocketError, SystemCallError, IOError,
      OpenSSL::SSL::SSLError, JSON::ParserError
    ].freeze

    # Google answers quota exhaustion as 429, or as 403 carrying one of
    # these reasons. Anything else on 403 is a permission problem.
    RATE_LIMIT_REASONS = %w[
      rateLimitExceeded userRateLimitExceeded
      quotaExceeded dailyLimitExceeded
    ].freeze

    class << self
      def configured?
        client_id.present? && client_secret.present?
      end

      def client_id
        ENV["GOOGLE_CLIENT_ID"].presence
      end

      def client_secret
        ENV["GOOGLE_CLIENT_SECRET"].presence
      end

      # The requested set is always complete (Calendar base plus the
      # optional Drive scope), so no incremental flag is sent: each grant
      # replaces the previous one, which is what sheds the retired
      # drive.metadata.readonly grant on reconnect.
      def authorize_url(redirect_uri:, state:, drive: false)
        uri = URI::HTTPS.build(host: AUTHORIZE_HOST, path: "/o/oauth2/v2/auth")
        params = {
          client_id: client_id, redirect_uri:, response_type: "code",
          scope: drive ? "#{SCOPE} #{DRIVE_SCOPE}" : SCOPE,
          access_type: "offline", prompt: "consent", state:
        }
        uri.query = URI.encode_www_form(params)
        uri.to_s
      end

      # Exchange an authorization code for tokens. Returns the parsed token
      # response (access_token, refresh_token, expires_in).
      def exchange_code(code:, redirect_uri:)
        response = post_token_form(
          client_id:, client_secret:, code:, redirect_uri:,
          grant_type: "authorization_code"
        )

        case response
        when Net::HTTPSuccess
          JSON.parse(response.body)
        else
          raise Error, "Google token exchange failed (#{response.code} #{token_error_code(response)})".squish
        end
      rescue *TRANSPORT_ERRORS => error
        raise Unavailable, "Google Calendar request failed (#{error.class})"
      end

      # Shared token-endpoint POST for the code exchange and refreshes.
      def post_token_form(params)
        uri = URI::HTTPS.build(host: TOKEN_HOST, path: "/token")
        Net::HTTP.start(uri.host, uri.port, use_ssl: true,
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          http.post(uri.request_uri, URI.encode_www_form(params),
            "Content-Type" => "application/x-www-form-urlencoded")
        end
      end

      def token_error_code(response)
        JSON.parse(response.body.to_s)["error"]
      rescue JSON::ParserError
        nil
      end

      # Best-effort grant revocation for disconnect and deactivation.
      # Returns true when Google accepted (or had already revoked) the
      # token, false on other client errors; transport failures, rate
      # limits, and 5xx raise Unavailable so the caller retries. Never
      # logs the token.
      def revoke_token(token)
        uri = URI::HTTPS.build(host: TOKEN_HOST, path: "/revoke")
        response = Net::HTTP.start(uri.host, uri.port, use_ssl: true,
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          http.post(uri.request_uri, URI.encode_www_form(token: token),
            "Content-Type" => "application/x-www-form-urlencoded")
        end

        if response.is_a?(Net::HTTPSuccess) || response.code == "400"
          true
        elsif response.code == "429" || response.code.start_with?("5")
          raise Unavailable, "Google token revoke failed (#{response.code})"
        else
          false
        end
      rescue *TRANSPORT_ERRORS => error
        raise Unavailable, "Google token revoke failed (#{error.class})"
      end

      # The account email comes from the id_token returned by the token
      # endpoint, so connecting needs no extra API call. The token arrives
      # directly from Google over TLS, so the payload is trusted after
      # checking iss/aud/exp; no signature check is needed.
      def email_from_id_token(id_token)
        segments = id_token.to_s.split(".")
        raise Error, "Google rejected the connection" unless segments.size == 3

        payload = JSON.parse(Base64.urlsafe_decode64(pad_base64url(segments[1])))
        raise Error, "Google rejected the connection" unless valid_id_token_payload?(payload)

        payload["email"]
      rescue ArgumentError, JSON::ParserError
        raise Error, "Google rejected the connection"
      end

      private
        def pad_base64url(segment)
          segment + "=" * (-segment.length % 4)
        end

        def valid_id_token_payload?(payload)
          payload.is_a?(Hash) &&
            payload["iss"].in?(ID_TOKEN_ISSUERS) &&
            payload["aud"] == client_id &&
            payload["exp"].to_i > Time.current.to_i &&
            payload["email"].present?
        end
    end

    def initialize(account)
      @account = account
    end

    def insert_event(payload)
      api_request(:post, "/calendar/v3/calendars/primary/events", payload)
    end

    def update_event(google_event_id, payload, conference_data_version: false)
      query = URI.encode_www_form(conferenceDataVersion: 1) if conference_data_version
      api_request(:put, "/calendar/v3/calendars/primary/events/#{google_event_id}", payload, query:)
    end

    # Partial update: only the supplied fields change. Conference
    # creation uses this: events.update replaces the whole resource and
    # requires start/end, so a conferenceData-only update is rejected
    # with 400, while events.patch accepts it:
    # https://developers.google.com/workspace/calendar/api/v3/reference/events/patch
    # https://developers.google.com/workspace/calendar/api/v3/reference/events/update
    def patch_event(google_event_id, payload, conference_data_version: false)
      query = URI.encode_www_form(conferenceDataVersion: 1) if conference_data_version
      api_request(:patch, "/calendar/v3/calendars/primary/events/#{google_event_id}", payload, query:)
    end

    def get_event(google_event_id)
      api_request(:get, "/calendar/v3/calendars/primary/events/#{google_event_id}")
    end

    # Timed event windows overlapping [time_min, time_max] for meeting
    # status and calendar out-of-office, under the existing calendar.events
    # grant (no new scope). The fields mask keeps titles, descriptions,
    # locations, and attendee identities out of the response entirely: only
    # each event's type, start/end, status, transparency, and every
    # attendee's self/declined flags arrive, and only the busy and OOO
    # intervals derived from them are kept.
    # https://developers.google.com/workspace/calendar/api/v3/reference/events/list
    MEETING_STATUS_FIELDS = "items(eventType,start,end,status,transparency,attendees(self,responseStatus))"

    def list_events(time_min:, time_max:)
      api_request(:get, "/calendar/v3/calendars/primary/events", nil,
        query: URI.encode_www_form(
          singleEvents: true, orderBy: "startTime", maxResults: 250,
          timeMin: time_min.iso8601, timeMax: time_max.iso8601,
          fields: MEETING_STATUS_FIELDS
        ))
    end

    # Opens a push channel (events.watch) on the primary calendar. Google
    # POSTs a sync handshake then one notification per change to address,
    # echoing token back in X-Goog-Channel-Token. Returns the parsed
    # watch response (resourceId, expiration in ms).
    def watch_events(channel_id:, token:, address:)
      api_request(:post, "/calendar/v3/calendars/primary/events/watch", {
        "id" => channel_id, "type" => "web_hook", "address" => address, "token" => token
      })
    end

    # Closes a push channel. A 404 means Google already dropped it, so it
    # counts as stopped.
    def stop_channel(channel_id:, resource_id:)
      api_request(:post, "/calendar/v3/channels/stop", {
        "id" => channel_id, "resourceId" => resource_id
      })
    rescue NotFound
      true
    end

    def delete_event(google_event_id)
      api_request(:delete, "/calendar/v3/calendars/primary/events/#{google_event_id}")
    end

    DRIVE_FILE_FIELDS = "id,name,mimeType,modifiedTime,owners(displayName),webViewLink,iconLink"

    # Viewer-side Drive metadata for link previews. Under the drive.file
    # grant a 403 means the viewer never picked this file with Smartfire
    # (or cannot open it), so it maps to NotFound just like a 404:
    # callers must not distinguish "no access" from "does not exist".
    def drive_file(file_id)
      api_request(:get, "/drive/v3/files/#{file_id}", nil,
        query: URI.encode_www_form(fields: DRIVE_FILE_FIELDS, supportsAllDrives: true),
        forbidden: :not_found, service_name: "Drive")
    end

    DRIVE_LIST_FIELDS = "files(id,name,mimeType,modifiedTime,owners(displayName),webViewLink)"

    # Viewer-side Drive search for the composer picker. Under the
    # drive.file grant only files the viewer opened with Smartfire are
    # visible. A blank query lists recent files; otherwise matches by
    # name. Single quotes and backslashes in the term are escaped per the
    # Drive query grammar.
    def list_drive_files(query:)
      drive_query = if query.to_s.present?
        "name contains '#{escape_drive_query(query.to_s)}' and trashed=false"
      else
        "trashed=false"
      end

      api_request(:get, "/drive/v3/files", nil,
        query: URI.encode_www_form(
          q: drive_query, pageSize: 10, fields: DRIVE_LIST_FIELDS,
          orderBy: "modifiedTime desc", spaces: "drive"
        ),
        forbidden: :not_found, service_name: "Drive")
    end

    def refresh_access_token!
      response = self.class.post_token_form(
        client_id: self.class.client_id, client_secret: self.class.client_secret,
        refresh_token: @account.refresh_token, grant_type: "refresh_token"
      )

      case response
      when Net::HTTPSuccess
        tokens = JSON.parse(response.body)
        @account.update!(
          access_token: tokens["access_token"],
          access_token_expires_at: Time.current + tokens["expires_in"].to_i.seconds
        )
      else
        if response.code == "400" && self.class.token_error_code(response) == "invalid_grant"
          @account.mark_disconnected!("Google rejected the connection")
          raise Unauthorized, "Google rejected the refresh token"
        end
        if response.code == "429"
          raise Unavailable, "Google token refresh rate limited (429)"
        end
        if response.code.start_with?("5")
          raise Unavailable, "Google token refresh failed (#{response.code})"
        end
        raise Error, "Google token refresh failed (#{response.code})"
      end
    rescue *TRANSPORT_ERRORS => error
      raise Unavailable, "Google Calendar request failed (#{error.class})"
    end

    private
      # Prefix each quote and backslash with a backslash, in one pass so the
      # added backslashes are never re-escaped. Block form: \' would be a
      # post-match backreference in a replacement string.
      def escape_drive_query(term)
        term.gsub(/['\\]/) { |char| "\\#{char}" }
      end

      # A Calendar 403 is either quota exhaustion (retryable) or a
      # permission problem (permanent): Google puts the distinction in
      # error.errors[].reason. Only matched constants reach the message,
      # never raw response bytes.
      def raise_for_calendar_forbidden!(response)
        if (reason = forbidden_reasons(response).find { |candidate| RATE_LIMIT_REASONS.include?(candidate) })
          raise RateLimited, "Google Calendar request rate limited (#{reason})"
        else
          raise Forbidden, "Google Calendar request forbidden (403)"
        end
      end

      def forbidden_reasons(response)
        JSON.parse(response.body.to_s).dig("error", "errors").to_a
          .filter_map { |entry| entry["reason"] if entry.is_a?(Hash) }
      rescue JSON::ParserError
        []
      end

      def api_request(method, path, payload = nil, query: nil, forbidden: :error, service_name: "Calendar")
        refresh_access_token! if @account.access_token_expired?

        response = send_api_request(method, path, payload, query)
        if response.code == "401"
          refresh_access_token!
          response = send_api_request(method, path, payload, query)
        end

        case response
        when Net::HTTPSuccess
          response.body.present? ? JSON.parse(response.body) : true
        when Net::HTTPUnauthorized
          raise Unauthorized, "Google rejected the request (401)"
        when Net::HTTPNotFound, Net::HTTPGone
          raise NotFound, "Google #{service_name.downcase} entry not found"
        when Net::HTTPForbidden
          if forbidden == :not_found
            raise NotFound, "Google #{service_name.downcase} entry not found"
          else
            raise_for_calendar_forbidden!(response)
          end
        when Net::HTTPTooManyRequests
          raise RateLimited, "Google #{service_name} request rate limited (429)"
        when Net::HTTPConflict
          raise Conflict, "Google calendar entry already exists"
        else
          raise Error, "Google #{service_name} request failed (#{response.code})"
        end
      rescue *TRANSPORT_ERRORS => error
        raise Unavailable, "Google #{service_name} request failed (#{error.class})"
      rescue ActiveRecord::Encryption::Errors::Decryption
        # The token rotted between the usable? check and this read (or a
        # caller skipped the check): fail as Unauthorized so Drive
        # endpoints 404 and sync drops the entry instead of retrying.
        @account.mark_disconnected!(GoogleAccount::UNREADABLE_TOKEN_REASON)
        raise Unauthorized, "Google token could not be read"
      end

      def send_api_request(method, path, payload, query = nil)
        uri = URI::HTTPS.build(host: API_HOST, path:, query:)
        Net::HTTP.start(uri.host, uri.port, use_ssl: true,
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          if method.in?(%i[ get delete ])
            http.send(method, uri.request_uri, headers)
          else
            http.send(method, uri.request_uri, payload&.to_json, headers)
          end
        end
      end

      def headers
        {
          "Content-Type" => "application/json",
          "Authorization" => "Bearer #{@account.access_token}"
        }
      end
  end
end
