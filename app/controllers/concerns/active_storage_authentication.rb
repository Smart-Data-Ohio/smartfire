module ActiveStorageAuthentication
  extend ActiveSupport::Concern
  include Authentication::SessionLookup

  private
    # These framework controllers never pass through ApplicationController,
    # so they repeat the session gate here: any human session must have
    # completed the second factor, matching TwoFactorEnforcement. Anything
    # else (anonymous, must-enroll, stale) is refused like an anonymous
    # caller, without revealing which.
    def require_active_storage_authentication
      session = find_session_by_cookie
      unless session&.two_factor_verified? || (session && !session.user.requires_two_factor?)
        head :unauthorized
      end
    end
end
