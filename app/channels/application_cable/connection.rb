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
        if verified_session = find_session_by_cookie
          if verified_session.expired?
            verified_session.destroy!
            reject_unauthorized_connection
          else
            @current_session = verified_session
            verified_session.user
          end
        else
          reject_unauthorized_connection
        end
      end
  end
end
