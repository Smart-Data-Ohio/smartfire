require "net/http"

module Slack
  # Read-only Slack Web API client behind the importer. One method per
  # endpoint the import needs, each citing its documentation page; every
  # call goes through #request, which paces Tier 2 and Tier 3 calls under
  # their documented limits, maps Slack's `ok: false` error codes, and
  # retries 5xx and network failures with backoff before raising.
  #
  # Requests are GET with query-string parameters and a user-token
  # Authorization header, per the method docs. The token is never logged.
  class Client
    API_HOST = "slack.com"
    OPEN_TIMEOUT = 10
    READ_TIMEOUT = 30

    # Internal apps keep Tier 3 limits (50+/min) on history/replies; Tier 2
    # list methods allow 20+/min. We stay a step under both.
    # See https://docs.slack.dev/changelog/2025/06/03/rate-limits-clarity/
    TIER2_MIN_INTERVAL = 60.0 / 18
    TIER3_MIN_INTERVAL = 60.0 / 45

    TIER2_METHODS = %w[ users.list conversations.list ].freeze

    MAX_ATTEMPTS = 4
    RETRY_BACKOFF = [ 1, 2, 4 ].freeze
    DEFAULT_RETRY_AFTER = 60

    class Error < StandardError; end
    # invalid_auth, token_revoked, account_inactive, not_authed. The run
    # fails and the connection is flagged disconnected.
    class AuthError < Error; end
    # missing_scope. Carries the scopes Slack said it needed.
    class ScopeError < Error
      attr_reader :needed, :provided

      def initialize(message, needed: nil, provided: nil)
        @needed = needed
        @provided = provided
        super(message)
      end
    end
    # HTTP 429 (or an ok:false ratelimited). Carries Retry-After seconds so
    # the job can reschedule itself instead of burning the retry budget.
    class RateLimited < Error
      attr_reader :retry_after

      def initialize(message, retry_after: DEFAULT_RETRY_AFTER)
        @retry_after = retry_after
        super(message)
      end
    end
    # Any other ok:false error. The run fails but stays resumable.
    class RequestError < Error; end
    # HTTP 5xx (and unmapped HTTP failures) or invalid JSON. Retried with
    # backoff a few times before surfacing as a RequestError.
    class HttpError < RequestError; end

    AUTH_ERRORS = %w[ invalid_auth token_revoked account_inactive not_authed ].freeze

    # on_request is called with the method name before every HTTP attempt
    # (retries included) so the runner can count API calls. Pacing sleeps
    # are disabled in tests (pass pacing: explicitly to override).
    def initialize(token:, on_request: nil, pacing: !Rails.env.test?)
      @token = token
      @on_request = on_request
      @pacing = pacing
      @last_call_at = {}
    end

    # See https://docs.slack.dev/reference/methods/auth.test
    def auth_test
      request("auth.test", {}, tier: :tier3)
    end

    # See https://docs.slack.dev/reference/methods/team.info
    def team_info
      request("team.info", {}, tier: :tier3)
    end

    # See https://docs.slack.dev/reference/methods/users.list
    def users_list(cursor: nil, limit: 200)
      request("users.list", { cursor:, limit: }, tier: :tier2)
    end

    # See https://docs.slack.dev/reference/methods/conversations.list
    def conversations_list(types:, cursor: nil, limit: 200)
      request("conversations.list",
        { types:, exclude_archived: "false", cursor:, limit: }, tier: :tier2)
    end

    # See https://docs.slack.dev/reference/methods/conversations.members
    # Tier 4 on paper; paced with the Tier 3 budget to be safe.
    def conversations_members(channel:, cursor: nil, limit: 1000)
      request("conversations.members", { channel:, cursor:, limit: }, tier: :tier3)
    end

    # See https://docs.slack.dev/reference/methods/conversations.history
    def conversations_history(channel:, oldest: nil, latest: nil, cursor: nil, limit: 200)
      request("conversations.history",
        { channel:, oldest:, latest:, cursor:, limit: }, tier: :tier3)
    end

    # See https://docs.slack.dev/reference/methods/conversations.replies
    def conversations_replies(channel:, ts:, oldest: nil, latest: nil, cursor: nil, limit: 200)
      request("conversations.replies",
        { channel:, ts:, oldest:, latest:, cursor:, limit: }, tier: :tier3)
    end

    private
      def request(method, params, tier:)
        attempts = 0

        begin
          attempts += 1
          pace(tier)
          payload = get(method, params.compact)
          check_ok!(payload, method)
        rescue RateLimited
          raise
        rescue HttpError, IOError, SystemCallError, Timeout::Error, SocketError => error
          if attempts >= MAX_ATTEMPTS
            raise error if error.is_a?(HttpError)

            raise RequestError, "Slack network error for #{method}: #{error.class}: #{error.message}"
          end

          pause(RETRY_BACKOFF[attempts - 1] || RETRY_BACKOFF.last)
          retry
        end
      end

      def get(method, params)
        @on_request&.call(method)
        uri = URI::HTTPS.build(host: API_HOST, path: "/api/#{method}",
          query: URI.encode_www_form(params))

        response = Net::HTTP.start(uri.host, uri.port, use_ssl: true,
          open_timeout: OPEN_TIMEOUT, read_timeout: READ_TIMEOUT) do |http|
          http.get(uri.request_uri, headers)
        end

        case response
        when Net::HTTPTooManyRequests
          raise RateLimited.new("Slack rate limited #{method}",
            retry_after: retry_after_from(response))
        when Net::HTTPSuccess
          parse_body(response.body, method)
        when Net::HTTPBadRequest, Net::HTTPUnauthorized, Net::HTTPForbidden, Net::HTTPNotFound
          # Slack usually answers 200 with ok:false, but map a JSON error
          # body on these the same way when one is present.
          payload = parse_body(response.body, method, raise_on_invalid: false)
          raise RequestError, "Slack HTTP #{response.code} for #{method}" if payload.nil?

          check_ok!(payload, method)
        else
          raise HttpError, "Slack HTTP #{response.code} for #{method}"
        end
      end

      def parse_body(body, method, raise_on_invalid: true)
        JSON.parse(body.to_s)
      rescue JSON::ParserError
        raise HttpError, "Slack returned invalid JSON for #{method}" if raise_on_invalid
      end

      def check_ok!(payload, method)
        return payload if payload["ok"]

        code = payload["error"].to_s
        if AUTH_ERRORS.include?(code)
          raise AuthError, "Slack authentication failed for #{method} (#{code})"
        elsif code == "missing_scope"
          needed = Array(payload["needed"]).join(",").presence || "unknown"
          raise ScopeError.new(
            "Slack token is missing a required scope for #{method} (needed: #{needed})",
            needed: payload["needed"], provided: payload["provided"])
        elsif code == "ratelimited"
          raise RateLimited, "Slack rate limited #{method}"
        else
          raise RequestError, "Slack error for #{method}: #{code.presence || "unknown"}"
        end
      end

      def retry_after_from(response)
        seconds = response["Retry-After"].to_i
        seconds.positive? ? seconds : DEFAULT_RETRY_AFTER
      end

      def headers
        {
          "Accept" => "application/json; charset=utf-8",
          "User-Agent" => "Smartfire-Slack-Import"
        }.tap do |headers|
          headers["Authorization"] = "Bearer #{@token}"
        end
      end

      def pace(tier)
        return unless @pacing

        interval = tier == :tier2 ? TIER2_MIN_INTERVAL : TIER3_MIN_INTERVAL
        now = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        if (last = @last_call_at[tier])
          wait = interval - (now - last)
          sleep(wait) if wait.positive?
          now = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        end
        @last_call_at[tier] = now
      end

      def pause(seconds)
        sleep(seconds) if @pacing
      end
  end
end
