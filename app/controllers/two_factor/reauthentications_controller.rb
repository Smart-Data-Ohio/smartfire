module TwoFactor
  # Step-up for members without a password (provisioned through Google):
  # proves the linked Google account again through the normal OAuth flow,
  # arming a single-use re-authentication for one sensitive self-service
  # action. The callback only accepts the SAME Google subject already
  # linked to the member. Password members can use it too when they would
  # rather not type their password.
  class ReauthenticationsController < ApplicationController
    include GoogleSignInFlow

    before_action :ensure_google_reauth_available

    def create
      redirect_to_google_sign_in(purpose: "reauth", user_id: Current.user.id)
    end

    private
      def ensure_google_reauth_available
        unless Google::SignIn.configured? && Current.user.google_identity.present?
          redirect_to user_profile_url, alert: "Google confirmation needs a linked Google account."
        end
      end
  end
end
