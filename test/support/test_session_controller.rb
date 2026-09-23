# Test-only fast sign-in for system tests. Loaded from config/routes.rb only
# when Rails.env.test?; the route does not exist in any other environment,
# and the action refuses to serve there even if it were somehow routed.
class TestSessionController < ApplicationController
  allow_unauthenticated_access only: :create

  before_action :ensure_test_environment, only: :create

  # Signs in with real credentials (the password is still verified) and a
  # real session row and cookie, skipping only the login form round-trips.
  # Served over GET so the browser helper reaches it with a plain visit;
  # CSRF only guards state-changing methods.
  def create
    if user = User.active.authenticate_by(email_address: params[:email_address], password: params[:password])
      start_new_session_for user
      redirect_to post_authenticating_url
    else
      render plain: "Unauthorized", status: :unauthorized
    end
  end

  private
    def ensure_test_environment
      raise ActionController::RoutingError, "Not Found" unless Rails.env.test?
    end
end
