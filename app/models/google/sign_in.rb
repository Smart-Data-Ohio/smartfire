module Google
  # Workspace "Sign in with Google" for ordinary members. Separate from
  # the Calendar/Drive connection flow (Google::Client), which requires
  # login and stays opt-in: this flow requests only "openid email
  # profile", never asks for offline access, and persists no OAuth
  # tokens -- only the verified subject in GoogleIdentity.
  module SignIn
    # Every deployment supplies its own Workspace domains. A missing or
    # empty allowlist disables Google sign-in.
    DOMAINS_ENV_VAR = "GOOGLE_SIGN_IN_DOMAINS"

    SCOPE = "openid email profile"
    AUTHORIZE_HOST = "accounts.google.com"
    JWKS_URI = "https://www.googleapis.com/oauth2/v3/certs"
    ISSUERS = %w[ https://accounts.google.com accounts.google.com ].freeze

    # How long a sign-in attempt stays usable between the POST that
    # starts it and Google's GET callback.
    FLOW_TTL = 10.minutes
    # Clock skew tolerated when checking token expiry.
    CLOCK_SKEW = 30.seconds

    # Step-up re-authentication -- "reauth" (TwoFactor::ReauthenticationsController)
    # and "sudo" (SudosController) -- must prove a FRESH Google login,
    # not ride an existing Google session: prompt=login with max_age=0
    # forces Google to authenticate the user again, and the id_token's
    # auth_time must be within FRESH_LOGIN_MAX_AUTH_AGE (see
    # IdTokenVerifier). Per OpenID Connect Core 1.0, when max_age is
    # requested the OP must re-authenticate past it and the ID Token
    # carries auth_time (https://openid.net/specs/openid-connect-core-1_0.html).
    # Normal sign-in and linking send neither prompt nor max_age and
    # never check auth_time.
    FRESH_LOGIN_PURPOSES = %w[ reauth sudo ].freeze
    FRESH_LOGIN_PROMPT = "login"
    FRESH_LOGIN_MAX_AGE = 0
    FRESH_LOGIN_MAX_AUTH_AGE = 5.minutes

    # Fail-closed errors. Messages are safe to log: they never carry
    # tokens, codes, or raw claim values.
    class Error < StandardError; end

    # Google could not be reached or answered with garbage: the user
    # can retry or fall back to their password.
    class Unavailable < Error; end

    # The attempt itself is invalid (bad state, bad token, wrong
    # domain, ineligible account). Carries a stable reason for the
    # controller to map onto a user-facing message.
    class Rejected < Error
      attr_reader :reason

      def initialize(reason)
        @reason = reason
        super("Google sign-in rejected (#{reason})")
      end
    end

    class << self
      def configured?
        Google::Client.configured? && allowed_domains.any?
      end

      # Normalized allowlist from GOOGLE_SIGN_IN_DOMAINS. Missing, empty,
      # or all-invalid configuration disables Google sign-in.
      def allowed_domains
        raw = ENV[DOMAINS_ENV_VAR]
        raw.to_s.split(",").map { |domain| domain.strip.downcase }
          .select { |domain| valid_domain?(domain) }.uniq
      end

      # prompt/max_age stay absent unless given: only step-up re-auth
      # ("reauth" and "sudo" purposes) sends prompt=login and max_age=0
      # (see GoogleSignInFlow), forcing a fresh Google login whose
      # auth_time the callback then checks.
      def authorize_url(redirect_uri:, state:, nonce:, challenge:, prompt: nil, max_age: nil)
        uri = URI::HTTPS.build(host: AUTHORIZE_HOST, path: "/o/oauth2/v2/auth")
        params = {
          client_id: Google::Client.client_id, redirect_uri:,
          response_type: "code", scope: SCOPE,
          state:, nonce:,
          code_challenge: challenge, code_challenge_method: "S256"
        }
        params[:prompt] = prompt if prompt.present?
        params[:max_age] = max_age unless max_age.nil?
        uri.query = URI.encode_www_form(params)
        uri.to_s
      end

      def pkce_pair
        verifier = SecureRandom.urlsafe_base64(32)
        challenge = Base64.urlsafe_encode64(Digest::SHA256.digest(verifier), padding: false)
        [ verifier, challenge ]
      end

      # Exchange the callback code for tokens and return only the
      # id_token. Tokens are never logged or persisted.
      def exchange_code(code:, redirect_uri:, verifier:)
        response = Google::Client.post_token_form(
          client_id: Google::Client.client_id,
          client_secret: Google::Client.client_secret,
          code:, redirect_uri:,
          grant_type: "authorization_code", code_verifier: verifier
        )

        case response
        when Net::HTTPSuccess
          payload = JSON.parse(response.body.to_s)
          raise Unavailable, "Google sign-in returned an invalid response" unless payload.is_a?(Hash)

          id_token = payload["id_token"]
          raise Rejected, :bad_token unless id_token.is_a?(String) && id_token.present?
          id_token
        else
          raise Rejected, :denied
        end
      rescue *Google::Client::TRANSPORT_ERRORS => error
        raise Unavailable, "Google sign-in request failed (#{error.class})"
      end

      # Same-origin guard for the post-auth return URL the existing
      # auth helper stashed in the session. Returns a safe path or
      # nil when the stored value is missing or points elsewhere.
      def safe_return_path(stored_url, host:)
        return nil if stored_url.blank?

        uri = URI.parse(stored_url.to_s)
        target = uri.relative? ? uri.to_s : (uri.request_uri if uri.host == host)

        if target.is_a?(String) && target.start_with?("/") && !target.start_with?("//", "/\\")
          target
        end
      rescue URI::InvalidURIError
        nil
      end

      private
        def valid_domain?(domain)
          domain.match?(/\A[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+\z/)
        end
    end
  end
end
