class Sessions::TransfersController < ApplicationController
  allow_unauthenticated_access

  def show
  end

  def update
    if user = User.active.find_by_transfer_id(params[:id])
      begin_session_for user, method: "transfer"
    else
      # The transfer id is a single-use credential: never log it.
      AuditLog.record_sign_in_failure!(email: "", method: "transfer")
      head :bad_request
    end
  end
end
