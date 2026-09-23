module ApplicationCable
  class Connection < ActionCable::Connection::Base
    include Authentication::SessionLookup

    identified_by :current_user
    attr_reader :current_session

    def connect
      self.current_user = find_verified_user
    end

    private
      def find_verified_user
        verified_session = find_session_by_cookie
        reject_unauthorized_connection unless verified_session

        if verified_session.expired?
          verified_session.destroy!
          reject_unauthorized_connection
        end

        @current_session = verified_session
        user = verified_session.user
        # Mirror the HTTP enforcement (TwoFactorEnforcement): a human
        # whose session never completed the second factor (must-enroll
        # or stale pre-2FA sessions) gets nothing over the socket
        # either. Both predicates read loaded attributes, so the gate
        # adds no queries to connection setup.
        reject_unauthorized_connection if user.requires_two_factor? && !verified_session.two_factor_verified?
        user
      end
  end
end
