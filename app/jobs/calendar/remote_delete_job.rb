class Calendar::RemoteDeleteJob < ApplicationJob
  # Deletes one orphaned Google copy after its local row was destroyed
  # outside the sync reconciler (room destroy, series rebuild). The
  # account row still exists, so credentials load fresh at run time; a
  # missing, rejected, or calendar-less account means the copy is
  # unreachable and there is nothing to do. Already-gone and revoked
  # copies are success; other Google failures are logged, since the
  # local row they could be recorded on is gone.
  retry_on Google::Client::Unavailable, wait: :polynomially_longer, attempts: 8

  def perform(user_id, google_event_id)
    account = User.find_by(id: user_id)&.google_account
    return unless account&.usable? && account.calendar?

    Google::Client.new(account).delete_event(google_event_id)
  rescue Google::Client::Unavailable
    raise
  rescue Google::Client::Error => error
    Rails.logger.warn "Calendar::RemoteDeleteJob could not remove #{google_event_id} for user #{user_id}: #{error.class}"
  end
end
