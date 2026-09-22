# Administrator controls for a member's Google sign-in link. Google
# sign-in links a first-time Google subject by email only to accounts
# whose email is known to be their own; an administrator vouches for the
# address here (create) so the next Google sign-in with it links.
# Unlinking (destroy) removes the stored Google subject, so that Google
# account no longer signs in as this member.
class Accounts::Users::GoogleLinksController < ApplicationController
  before_action :ensure_can_administer
  before_action :set_user

  def create
    @user.update!(email_self_changed_at: nil, google_email_link_allowed: true)
    redirect_to edit_account_url, notice: "#{@user.name} can now link Google sign-in for #{@user.email_address}."
  end

  def destroy
    @user.google_identity&.destroy!
    redirect_to edit_account_url, notice: "Google sign-in unlinked from #{@user.name}."
  end

  private
    def set_user
      @user = User.active.without_bots.find(params[:user_id])
    end
end
