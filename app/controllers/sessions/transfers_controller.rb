class Sessions::TransfersController < ApplicationController
  allow_unauthenticated_access

  def show
  end

  def update
    if user = User.active.find_by_transfer_id(params[:id])
      start_new_session_for user
      AuditLog.record!(action: "session.sign_in.success", actor: user, changes: { method: "transfer" })
      redirect_to post_authenticating_url
    else
      # The transfer id is a single-use credential: never log it.
      AuditLog.record_sign_in_failure!(email: "", method: "transfer")
      head :bad_request
    end
  end
end
