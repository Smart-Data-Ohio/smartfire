class SessionsController < ApplicationController
  allow_unauthenticated_access only: %i[ new create ]
  rate_limit to: 10, within: 3.minutes, only: :create, with: -> { render_rejection :too_many_requests }

  before_action :ensure_user_exists, only: :new

  def new
    # Background polls redirected to sign in keep their JSON Accept
    # header; answer 401 instead of raising UnknownFormat, so the fetch
    # fails quietly like any other signed-out request.
    respond_to do |format|
      format.html
      format.json { head :unauthorized }
    end
  end

  def create
    if user = User.active.authenticate_by(email_address: params[:email_address], password: params[:password])
      begin_session_for user, method: "password"
    else
      render_rejection :unauthorized
    end
  end

  def destroy
    remove_push_subscription
    terminate_current_session
    redirect_to root_url
  end

  private
    def ensure_user_exists
      redirect_to first_run_url if User.none?
    end

    # Both wrong-password (401) and rate-limited (429) attempts land here.
    # Failures are throttled per IP inside the audit log so a
    # credential-stuffing flood leaves one row instead of thousands.
    def render_rejection(status)
      AuditLog.record_sign_in_failure!(email: params[:email_address].to_s, method: "password")
      flash.now[:alert] = "Too many requests or unauthorized."
      render :new, status: status
    end

    def remove_push_subscription
      if endpoint = params[:push_subscription_endpoint]
        Push::Subscription.destroy_by(endpoint: endpoint, user_id: Current.user.id)
      end
    end
end
