# "Link Google sign-in" on a member's own profile: runs the Google sign-in
# flow while they are signed in, and Sessions::GoogleController#callback
# links the verified Google subject to them. Safe without an email match
# because the member proves the account is theirs by being signed in.
class Users::GoogleSignInLinksController < ApplicationController
  include GoogleSignInFlow

  before_action :ensure_configured

  def create
    if Current.user.google_identity
      redirect_to user_profile_url, notice: "Google sign-in is already linked."
    else
      redirect_to_google_sign_in(purpose: "link", user_id: Current.user.id)
    end
  end

  private
    def ensure_configured
      head :not_found unless Google::SignIn.configured?
    end
end
