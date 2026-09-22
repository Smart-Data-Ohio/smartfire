module AgentApiThrottle
  extend ActiveSupport::Concern

  class_methods do
    # Minute-bucketed throttle keyed by the calling credential, so one
    # busy credential cannot starve others sharing the agent. Overflowing
    # the bucket renders 429 with a Retry-After. Human session requests
    # are not throttled here. Backed by Rails.cache, so it only bites
    # when a real cache store is configured.
    def throttle_agent_api(limit:, only:)
      before_action(only: only) { check_agent_api_throttle(limit) }
    end
  end

  private
    def check_agent_api_throttle(limit)
      key = agent_throttle_key
      return if key.nil? || performed?

      window = 1.minute
      bucket = Time.current.to_i / window.to_i
      count = Rails.cache.increment(
        "agent_api_throttle:#{controller_path}:#{action_name}:#{bucket}:#{key}",
        1, expires_in: window + 5.seconds
      ).to_i
      return if count <= limit

      response.set_header("Retry-After", [ (bucket + 1) * window.to_i - Time.current.to_i, 1 ].max.to_s)
      render json: { error: "rate_limited" }, status: :too_many_requests
    end

    # The credential behind an agent-authenticated request, identified by
    # its token digest without a database query. Requests without
    # bearer-token auth (human sessions) return nil and skip the throttle.
    def agent_throttle_key
      authorization = request.authorization.to_s
      return unless authorization.match?(/\ABearer /i)

      token = authorization.sub(/\ABearer /i, "").strip
      token.presence && AgentCredential.digest(token)
    end
end
