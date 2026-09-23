module Authentication
  extend ActiveSupport::Concern
  include SessionLookup

  included do
    before_action :require_authentication
    before_action :deny_bots
    before_action :deny_agent_tokens
    helper_method :signed_in?

    protect_from_forgery with: :exception, unless: -> { authenticated_by.bot_key? || authenticated_by.bot_reply? || authenticated_by.agent_token? }
  end

  class_methods do
    def allow_unauthenticated_access(**options)
      skip_before_action :require_authentication, **options
    end

    def allow_bot_access(**options)
      skip_before_action :deny_bots, **options
    end

    def allow_agent_access(**options)
      skip_before_action :deny_agent_tokens, **options
    end

    def require_unauthenticated_access(**options)
      skip_before_action :require_authentication, **options
      before_action :restore_authentication, :redirect_signed_in_user_to_root, **options
    end
  end

  private
    def signed_in?
      Current.user.present?
    end

    def require_authentication
      restore_authentication || bot_authentication || agent_authentication || request_authentication
    end

    def restore_authentication
      if session = find_session_by_cookie
        if session.expired?
          session.destroy!
          remove_authentication_cookie
          nil
        else
          resume_session session
        end
      end
    end

    def bot_authentication
      return if params[:bot_key].blank?

      key = params[:bot_key].strip
      if (bot = User.authenticate_bot(key))
        Current.user = bot
        set_authenticated_by(:bot_key)
      elsif (bot = User.authenticate_bot_reply_token(key, room_id: params[:room_id]))
        Current.user = bot
        set_authenticated_by(:bot_reply)
      end
    end

    def agent_authentication
      return unless agent_bearer_secret

      credential = AgentCredential.authenticate(agent_bearer_secret)

      if credential&.agent&.active?
        Current.agent = credential.agent
        Current.user = credential.agent.user
        credential.record_use!(request.remote_ip)
        credential.agent.touch_last_seen!
        set_authenticated_by(:agent_token)
      else
        head :unauthorized
        true
      end
    end

    def agent_bearer_secret
      return @agent_bearer_secret if defined?(@agent_bearer_secret)

      scheme, token = request.authorization.to_s.split(" ", 2)
      @agent_bearer_secret = if scheme&.casecmp?("Bearer") && token.present?
        token.strip.presence
      end
    end

    def request_authentication
      session[:return_to_after_authenticating] = request.url
      redirect_to new_session_url
    end

    def redirect_signed_in_user_to_root
      redirect_to root_url if signed_in?
    end

    def start_new_session_for(user)
      # A new session starts unverified: a different member signing in on
      # this browser must not inherit the previous user's confirmation.
      session.delete(SudoMode::VERIFIED_SESSION_KEY)
      session.delete(SudoMode::PENDING_SESSION_KEY)

      device_id = ensure_device_cookie
      user.sessions.start!(user_agent: request.user_agent, ip_address: request.remote_ip, device_id: device_id).tap do |session|
        authenticated_as session

        # Establish the CSRF token before any page renders. Sign-ins that
        # skip rendering a form (test helper, OAuth, first run, invites)
        # would otherwise leave the session without one, and then the first
        # concurrent page + sidebar renders each generate their own token;
        # the sidebar's commit lands last and invalidates the page meta, so
        # the next PATCH/POST 422s. Reading the token here commits it with
        # the sign-in response, so every later request reuses it.
        form_authenticity_token

        NewSignInAlert.deliver_if_new_device(user, session)
      end
    end

    # The stable device identifier new-device sign-in alerts are keyed on:
    # a long-lived signed cookie, not the IP alone. Every sign-in either
    # reuses the browser's id or mints one, so a sign-in from a browser the
    # account has never used is recognizable. The w4-two-factor branch's
    # remember-device cookie is a separate concern; keep the names apart.
    # HttpOnly and SameSite=Lax, like the session token: only sign-in
    # reads it, so scripts never need it and it must not ride
    # cross-site requests (a stolen id would let an attacker reuse a
    # known device and skip the new-device alert).
    def ensure_device_cookie
      cookies.signed[:device_id] || begin
        device_id = SecureRandom.hex(16)
        cookies.signed.permanent[:device_id] = { value: device_id, httponly: true, same_site: :lax }
        device_id
      end
    end

    def resume_session(session)
      session.resume user_agent: request.user_agent, ip_address: request.remote_ip
      authenticated_as session
    end

    def terminate_current_session
      Current.session&.destroy!
      reset_session
      remove_authentication_cookie
      disconnect_remote_connections
    end

    def disconnect_remote_connections
      Current.user&.reset_remote_connections
    rescue => error
      Rails.logger.warn "Could not disconnect remote connections on sign out: #{error.class}"
    end

    def authenticated_as(session)
      Current.session = session
      set_authenticated_by(:session)
      set_authentication_cookie(session)
    end

    def post_authenticating_url
      session.delete(:return_to_after_authenticating) || root_url
    end

    def set_authentication_cookie(session)
      cookies.signed.permanent[:session_token] = { value: session.token, httponly: true, same_site: :lax }
    end

    def remove_authentication_cookie
      cookies.delete(:session_token)
    end

    def deny_bots
      head :forbidden if authenticated_by.bot_key? || authenticated_by.bot_reply?
    end

    def deny_agent_tokens
      head :forbidden if authenticated_by.agent_token?
    end

    def set_authenticated_by(method)
      @authenticated_by = method.to_s.inquiry
    end

    def authenticated_by
      @authenticated_by ||= "".inquiry
    end
end
