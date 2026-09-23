# Step-up authentication for sensitive two-factor self-service:
# disabling, backup-code regeneration, and remembered-device revocation.
# Each action requires either the current TOTP code or the account
# password in the same request. Members without a password (provisioned
# through Google) can instead complete a Google re-auth, which arms a
# single-use step-up good for exactly one action.
module TwoFactorReauthentication
  extend ActiveSupport::Concern

  REAUTH_PARAM = :reauth
  REAUTH_SESSION_KEY = :two_factor_reauthenticated_at
  REAUTH_TTL = 10.minutes

  private
    def reauthenticated?(user)
      if params[REAUTH_PARAM].present?
        totp_or_password_confirmed?(user, params[REAUTH_PARAM].to_s)
      else
        consume_google_reauthentication!
      end
    end

    # A wrong in-request credential refuses without consuming a pending
    # Google step-up: the next attempt may still use it. Only numeric input
    # reaches the TOTP verifier, so passwords skip straight to the password
    # check and backup codes never count as re-authentication.
    def totp_or_password_confirmed?(user, value)
      return false if value.blank?

      credential = user.two_factor_credential
      ((credential&.enabled? && code_shaped?(value) && credential.verify_code(value)) ||
        user.authenticate(value).present?)
    end

    def code_shaped?(value)
      value.match?(/\A[\d\s]{6,10}\z/)
    end

    # Single-use: a completed Google re-auth arms exactly one sensitive
    # action, and expires after REAUTH_TTL either way.
    def consume_google_reauthentication!
      confirmed_at = session.delete(REAUTH_SESSION_KEY)
      confirmed_at.present? && confirmed_at.to_i > REAUTH_TTL.ago.to_i
    end

    def refuse_without_reauthentication(user)
      alert = if user.password_digest.blank?
        "Enter your authenticator code or confirm with Google to continue."
      else
        "Enter your authenticator code or password to continue."
      end
      redirect_to user_profile_url, alert: alert
    end
end
