# Sudo mode: sensitive actions require the member to have confirmed
# their identity within SUDO_TIMEOUT. Controllers opt in with
# `before_action :require_sudo_mode` (after their authorization
# checks, so a forbidden or unconfigured action never prompts).
#
# The UX is one prompt, then the action continues: the intercepted
# request is stashed in the session, the member confirms once at
# /sudo/new, and then GETs redirect straight back to the stashed path
# while small non-GETs render an auto-submitting replay form
# (sudos/continue) carrying the stashed scalar params with a fresh CSRF
# token. Requests with secrets, uploads, or large bodies are never
# stashed; those return to the originating page after confirmation and
# the member resubmits once.
#
# Verification methods (see docs/security.md#sudo-mode):
# - :password for members with a password;
# - :google re-auth for members with a linked Google identity
#   (Google-only members have no password to check);
# - :totp once the w4-two-factor branch lands: it calls
#   SudoMode.register_verifier(:totp) from an initializer, reopens
#   SudoMode.verify_totp to check the code, reopens
#   SudoMode.verifier_available? to gate on enrollment, and adds the
#   form partial referenced from sudos/new.
module SudoMode
  extend ActiveSupport::Concern

  SUDO_TIMEOUT = 15.minutes
  VERIFIED_SESSION_KEY = :sudo_verified_at
  PENDING_SESSION_KEY = :sudo_pending_request

  # Stored-request params whose keys match are never replayed: replaying
  # would stash a credential in the session. Those actions return to
  # their originating page after confirmation instead.
  SECRET_PARAM_PATTERN = /passw|passwd|pwd|secret|token|api[-_]?key|_key\z|credential|authorization|cookie|session|join[-_]?code|webhook_url|access_token/i
  MAX_STORED_PARAMS_BYTES = 2048

  @extra_verifiers = []

  class << self
    attr_reader :extra_verifiers

    def register_verifier(name)
      @extra_verifiers |= [ name.to_sym ]
    end

    # True when the verifier confirms the user, false when it rejects,
    # :unsupported when no implementation is loaded.
    def verify_with(verifier, user, params)
      case verifier.to_sym
      when :password
        verify_password(user, params[:password])
      when :totp
        verify_totp(user, params[:totp_code])
      else
        :unsupported
      end
    end

    def verify_password(user, password)
      user.password_digest.present? && user.authenticate(password.to_s).present?
    end

    # Hook point for w4-two-factor (see above): reopens to check the
    # TOTP code against the member's enrolled secret.
    def verify_totp(user, code)
      :unsupported
    end

    # Hook point for w4-two-factor: reopens so :totp is offered only to
    # members with two-step sign-in enrolled.
    def verifier_available?(name, user)
      false
    end
  end

  private
    def require_sudo_mode
      return if sudo_verified?

      store_sudo_pending_request
      redirect_to new_sudo_url
    end

    def sudo_verified?
      verified_at = session[SudoMode::VERIFIED_SESSION_KEY]
      verified_at.is_a?(Integer) && Time.at(verified_at) > SudoMode::SUDO_TIMEOUT.ago
    end

    def mark_sudo_verified!
      session[SudoMode::VERIFIED_SESSION_KEY] = Time.current.to_i
    end

    def sudo_verifiers_for(user)
      verifiers = []
      verifiers << :password if user.password_digest.present?
      verifiers << :google if user.google_identity.present? && Google::SignIn.configured?
      SudoMode.extra_verifiers.each do |name|
        verifiers << name if SudoMode.verifier_available?(name, user)
      end
      verifiers
    end

    def store_sudo_pending_request
      session[SudoMode::PENDING_SESSION_KEY] = {
        "method" => request.request_method,
        "path" => request.fullpath,
        "params" => sudo_storable_params,
        "origin" => sudo_origin_path
      }
    end

    # The request's scalar params for replay, or nil when there is
    # nothing safe to replay (GETs, secrets, uploads, oversized).
    def sudo_storable_params
      return nil unless request.post? || request.patch? || request.put? || request.delete?

      filtered = request.request_parameters.except("controller", "action", "authenticity_token")
      return nil unless sudo_scalar_params?(filtered)
      return nil if filtered.to_json.bytesize > SudoMode::MAX_STORED_PARAMS_BYTES

      filtered.as_json
    end

    def sudo_scalar_params?(value, depth = 0)
      return false if depth > 4

      case value
      when Hash
        value.keys.all?(String) &&
          value.keys.none? { |key| key.match?(SudoMode::SECRET_PARAM_PATTERN) } &&
          value.values.all? { |entry| sudo_scalar_params?(entry, depth + 1) }
      when Array
        value.all? { |entry| sudo_scalar_params?(entry, depth + 1) }
      when String, Integer, Float, TrueClass, FalseClass, NilClass
        true
      else
        false
      end
    end

    # Where a non-replayable request resumes: the same-host referrer, or
    # the workspace root when there is none.
    def sudo_origin_path
      if request.referer.present?
        uri = URI.parse(request.referer)
        return uri.path.presence || root_path if uri.host == request.host
      end
      root_path
    rescue URI::InvalidURIError
      root_path
    end

    # Runs after a successful confirmation. GETs redirect back to the
    # stashed path so the action continues automatically; stashed
    # non-GETs render the replay form; anything without a safe stashed
    # path returns to the originating page.
    def continue_after_sudo!
      pending = session.delete(SudoMode::PENDING_SESSION_KEY) || {}
      path = pending["path"].to_s
      path = nil unless path.start_with?("/") && !path.start_with?("//")

      if path && pending["method"].to_s.upcase == "GET"
        redirect_to path
      elsif path && pending["params"]
        @sudo_replay_method = pending["method"].to_s.downcase
        @sudo_replay_path = path
        @sudo_replay_params = pending["params"]
        render "sudos/continue"
      else
        redirect_to pending["origin"].presence || root_path
      end
    end
end
