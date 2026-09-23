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

  TWO_FACTOR_PENDING_USER_KEY = :two_factor_pending_user_id
  TWO_FACTOR_PENDING_EXPIRY_KEY = :two_factor_pending_expires_at
  TWO_FACTOR_PENDING_METHOD_KEY = :two_factor_pending_method
  TWO_FACTOR_PENDING_TTL = 10.minutes
  TWO_FACTOR_REMEMBER_COOKIE = :two_factor_remember

  private
    def signed_in?
      Current.user.present?
    end

    def require_authentication
      restore_authentication || bot_authentication || agent_authentication || request_authentication
    end

    def restore_authentication
      if session = find_session_by_cookie
        resume_session session
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
      if two_factor_pending_user.present?
        redirect_to two_factor_challenge_url
      else
        session[:return_to_after_authenticating] = request.url
        redirect_to new_session_url
      end
    end

    def redirect_signed_in_user_to_root
      redirect_to root_url if signed_in?
    end

    def start_new_session_for(user, two_factor_verified: false)
      user.sessions.start!(user_agent: request.user_agent, ip_address: request.remote_ip, two_factor_verified: two_factor_verified).tap do |session|
        authenticated_as session

        # Establish the CSRF token before any page renders. Sign-ins that
        # skip rendering a form (test helper, OAuth, first run, invites)
        # would otherwise leave the session without one, and then the first
        # concurrent page + sidebar renders each generate their own token;
        # the sidebar's commit lands last and invalidates the page meta, so
        # the next PATCH/POST 422s. Reading the token here commits it with
        # the sign-in response, so every later request reuses it.
        form_authenticity_token
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

    # Finishes the first factor (password, Google, transfer) for a human
    # user. Enrolled users either ride a valid remember-device cookie
    # straight in or wait in the pending state for the challenge; no real
    # session exists until the code is verified. Unenrolled users get a
    # plain session and the enrollment enforcement sends them to setup.
    def begin_session_for(user, method:, return_url: nil)
      return_url ||= post_authenticating_url

      if user.two_factor_enabled?
        if TwoFactorRememberedDevice.find_valid(cookies.signed[TWO_FACTOR_REMEMBER_COOKIE], user)
          start_new_session_for user, two_factor_verified: true
          AuditLog.record!(action: "session.sign_in.success", actor: user,
            changes: { method: method, two_factor: "remembered_device" })
          redirect_to return_url
        else
          session[:return_to_after_authenticating] = return_url
          stash_two_factor_pending(user, method)
          redirect_to two_factor_challenge_url
        end
      else
        start_new_session_for user
        AuditLog.record!(action: "session.sign_in.success", actor: user, changes: { method: method })
        redirect_to return_url
      end
    end

    # The pending second-factor state: a user id plus an expiry in the
    # encrypted cookie session. It grants nothing (Current.user stays
    # nil); it only names who may attempt the challenge.
    def stash_two_factor_pending(user, method)
      session[TWO_FACTOR_PENDING_USER_KEY] = user.id
      session[TWO_FACTOR_PENDING_EXPIRY_KEY] = TWO_FACTOR_PENDING_TTL.from_now.to_i
      session[TWO_FACTOR_PENDING_METHOD_KEY] = method
    end

    def two_factor_pending_method
      session[TWO_FACTOR_PENDING_METHOD_KEY].to_s.presence || "unknown"
    end

    def two_factor_pending_user
      user_id = session[TWO_FACTOR_PENDING_USER_KEY]
      expires_at = session[TWO_FACTOR_PENDING_EXPIRY_KEY]
      return nil if user_id.blank? || expires_at.blank? || expires_at.to_i < Time.current.to_i

      User.active.find_by(id: user_id)
    end

    def clear_two_factor_pending!
      session.delete(TWO_FACTOR_PENDING_USER_KEY)
      session.delete(TWO_FACTOR_PENDING_EXPIRY_KEY)
      session.delete(TWO_FACTOR_PENDING_METHOD_KEY)
    end

    # A signed, httponly, secure cookie bound to a revocable server-side
    # record. It survives sign-out by design; revocation deletes the row.
    def remember_two_factor_device!(user)
      _device, token = TwoFactorRememberedDevice.create_for!(user,
        user_agent: request.user_agent, ip_address: request.remote_ip)
      cookies.signed[TWO_FACTOR_REMEMBER_COOKIE] = {
        value: token, expires: TwoFactorRememberedDevice::REMEMBER_FOR,
        httponly: true, secure: true, same_site: :lax
      }
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
