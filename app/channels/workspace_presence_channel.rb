class WorkspacePresenceChannel < ApplicationCable::Channel
  def subscribed
    @lease = WorkspacePresenceLease.establish(user: current_user, session: connection.current_session)
    reject unless @lease
  end

  def unsubscribed
    @lease&.delete
  end

  def heartbeat(data = {})
    expire_idle_timed_out_session!

    return if @lease&.refresh(active: data["active"] == true)

    @lease = WorkspacePresenceLease.establish(user: current_user, session: connection.current_session)
    reject unless @lease
  end

  private
    # An administrator session that hit the idle timeout dies here, on
    # the next heartbeat, instead of lingering on the already-open
    # cable. Destroying it first reuses the revocation path above (the
    # lease refresh deletes the lease, re-establishing fails, the
    # subscription is rejected), following the connection's
    # expired-check-then-destroy pattern. A fresh read, never the
    # connect-time snapshot, and a revoked session simply skips.
    def expire_idle_timed_out_session!
      session = connection.current_session
      fresh = session && Session.find_by(id: session.id)
      fresh.destroy! if fresh&.expired?
    end
end
