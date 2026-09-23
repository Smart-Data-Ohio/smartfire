# The browser-bound half of Google sign-in shared by signing in
# (Sessions::GoogleController) and linking Google to a signed-in member's
# own account (Users::GoogleSignInLinksController): a one-use state,
# nonce, and PKCE verifier kept in the session, and the redirect to
# Google. Both flows return to the same registered callback URL, which
# tells them apart by the flow's purpose.
module GoogleSignInFlow
  extend ActiveSupport::Concern

  FLOW_SESSION_KEY = :google_sign_in_request

  private
    def redirect_to_google_sign_in(purpose:, **flow_attributes)
      verifier, challenge = Google::SignIn.pkce_pair
      raw_state = SecureRandom.hex(16)

      session[FLOW_SESSION_KEY] = {
        "state" => raw_state,
        "nonce" => SecureRandom.hex(16),
        "verifier" => verifier,
        "exp" => Google::SignIn::FLOW_TTL.from_now.to_i,
        "purpose" => purpose
      }.merge(flow_attributes.stringify_keys)

      redirect_to Google::SignIn.authorize_url(
        redirect_uri: session_google_callback_url,
        state: google_sign_in_state_verifier.generate(raw_state),
        nonce: session[FLOW_SESSION_KEY]["nonce"],
        challenge:,
        **fresh_login_authorize_params(purpose)
      ), allow_other_host: true
    end

    # Step-up re-authentication ("reauth" for two-step self-service,
    # "sudo" for sudo mode) must prove a FRESH Google login, not reuse
    # an existing Google session: prompt=login forces the login screen
    # and max_age=0 asks Google to report when it happened (auth_time),
    # which the callback verifies is minutes old. Sign-in and linking
    # send neither.
    def fresh_login_authorize_params(purpose)
      if Google::SignIn::FRESH_LOGIN_PURPOSES.include?(purpose.to_s)
        { prompt: Google::SignIn::FRESH_LOGIN_PROMPT, max_age: Google::SignIn::FRESH_LOGIN_MAX_AGE }
      else
        {}
      end
    end

    def google_sign_in_state_verifier
      Rails.application.message_verifier("google_sign_in_state")
    end
end
