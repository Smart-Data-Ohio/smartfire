# A member's own active sessions ("Your sessions" on the profile):
# device, browser, last active time, and last IP. Revoking destroys the
# row and drops every cable connection, with reconnect on: the revoked
# browser's sockets reconnect and are rejected at connect (the session
# row is gone), while surviving sessions resubscribe. Every revocation
# is audit-logged.
class Users::SessionsController < ApplicationController
  before_action :no_store_response!

  def index
    @sessions = Current.user.sessions.order(last_active_at: :desc)
  end

  def destroy
    session = Current.user.sessions.find(params[:id])

    if session == Current.session
      remove_push_subscription
      terminate_current_session
      redirect_to root_url
    else
      session_id = session.id
      session.destroy!
      Current.user.reset_remote_connections
      AuditLog.record!(action: "session.revoke", target: Current.user, changes: { revoked_session_id: session_id })
      redirect_to user_sessions_url, notice: "Signed out that session."
    end
  end

  def revoke_others
    others = Current.user.sessions.where.not(id: Current.session.id)
    count = others.count
    # A no-op revocation writes no audit row and drops no connections,
    # like every other idempotent revocation in the app.
    if count.zero?
      return redirect_to user_sessions_url, notice: "No other sessions to sign out."
    end

    others.destroy_all
    Current.user.reset_remote_connections
    AuditLog.record!(action: "session.revoke_others", target: Current.user, changes: { count: count })
    redirect_to user_sessions_url, notice: count == 1 ? "Signed out #{count} other session." : "Signed out #{count} other sessions."
  end

  private
    def remove_push_subscription
      if endpoint = params[:push_subscription_endpoint]
        Push::Subscription.destroy_by(endpoint: endpoint, user_id: Current.user.id)
      end
    end
end
