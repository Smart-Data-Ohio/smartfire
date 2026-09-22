class Rooms::MembersController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  def index
    members = @room.users.active.with_attached_avatar.includes(:agent).order(Arel.sql("LOWER(users.name) ASC"), :id).to_a
    online_user_ids = WorkspacePresenceLease.online_user_ids(members.map(&:id)).to_set

    render json: {
      members: members.map { |member| member_json(member, online_user_ids:) }
    }
  end

  private
    def request_authentication
      request.format.json? ? head(:unauthorized) : super
    end

    def member_json(member, online_user_ids:)
      {
        id: member.id,
        name: member.name,
        avatar_url: fresh_user_avatar_url(member),
        online: member_online?(member, online_user_ids)
      }
    end

    # Bots hold no presence lease, so a bot with an agent reads live from
    # the agent instead, like the agent profiles do: not suspended, and
    # checked in at least once. Members here are already active-scoped.
    def member_online?(member, online_user_ids)
      agent = member.agent

      if member.bot? && agent
        agent.suspended_at.nil? && agent.last_seen_at.present?
      else
        online_user_ids.include?(member.id)
      end
    end
end
