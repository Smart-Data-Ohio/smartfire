# Serves the shared profile-card popover: one turbo-frame per page, loaded
# lazily from any avatar or display name. Any signed-in member may view any
# user's card; the actions inside re-check their own authorization (the DM
# endpoints scope to membership, huddle issuance to humans).
class Users::CardsController < ApplicationController
  def show
    @user = User.includes(agent: :owner).with_attached_avatar.find(params[:id])
    @online = @user.online_now?

    render layout: false
  end
end
