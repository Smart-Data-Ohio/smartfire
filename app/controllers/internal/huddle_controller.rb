class Internal::HuddleController < ActionController::API
  before_action :prevent_caching
  before_action :authenticate_gateway
  before_action :ensure_huddles_configured

  def authorize
    coordinates = Huddle::TokenVerifier.new(bearer_token).coordinates
    grant = HuddleGrant.find_by(**coordinates)

    if grant&.authorize_or_revoke!
      grant.record_seen!
      render json: grant.authorization_payload
    else
      head :forbidden
    end
  rescue Huddle::TokenVerifier::Invalid
    head :unauthorized
  end

  def show
    grant = HuddleGrant.find_by(id: params[:id])

    if grant&.authorize_or_revoke!
      grant.record_seen!
      render json: grant.authorization_payload
    else
      head :not_found
    end
  end

  # The gateway posts here after its reconnect grace expires with no
  # replacement connection: the participant is gone, so the grant drops out
  # of the call and presence refreshes immediately instead of waiting out
  # the liveness window and the browser poll. Best effort on both sides —
  # enforcement never depends on it — so an unknown grant is a plain 404.
  def left
    grant = HuddleGrant.find_by(id: params[:id])
    return head :not_found unless grant

    floor = disconnected_at_param
    return head :unprocessable_entity if params[:disconnected_at].present? && floor.nil?

    grant.mark_out_of_call!(seen_after: floor)
    head :ok
  end

  private
    # The gateway sends ISO 8601 or nothing. A present-but-unparseable
    # timestamp is a client bug, not a clear, and out-of-range values make
    # the parser raise rather than return nil.
    def disconnected_at_param
      raw = params[:disconnected_at].to_s
      return if raw.blank?

      Time.zone.parse(raw)
    rescue ArgumentError
      nil
    end
    def authenticate_gateway
      provided = request.headers["X-Huddle-Gateway-Secret"].to_s
      expected = ENV["LIVEKIT_GATEWAY_SECRET"].to_s

      head :unauthorized unless provided.present? && expected.present? &&
        ActiveSupport::SecurityUtils.secure_compare(provided, expected)
    end

    def ensure_huddles_configured
      head :service_unavailable unless Huddle.configured?
    end

    def bearer_token
      scheme, token = request.authorization.to_s.split(" ", 2)
      raise Huddle::TokenVerifier::Invalid unless scheme&.casecmp?("Bearer") && token.present?

      token
    end

    def prevent_caching
      response.headers["Cache-Control"] = "no-store"
    end
end
