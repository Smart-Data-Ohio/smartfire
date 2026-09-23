# Serves the shared profile-card popover: one turbo-frame per page, loaded
# lazily from any avatar or display name. Any signed-in member may view any
# user's card; the actions inside re-check their own authorization (the DM
# endpoints scope to membership, huddle issuance to humans).
class Users::CardsController < ApplicationController
  def show
    @user = User.includes(agent: :owner).with_attached_avatar.find(params[:id])
    @online = online?(@user)

    render layout: false
  end

  private
    # Same rule as the member panel: bots hold no presence lease, so a bot
    # with an agent reads live from the agent instead.
    def online?(user)
      return false unless user.active?

      if user.bot? && user.agent
        user.agent.suspended_at.nil? && user.agent.last_seen_at.present?
      else
        WorkspacePresenceLease.online_user_ids([ user.id ]).include?(user.id)
      end
    end
end
