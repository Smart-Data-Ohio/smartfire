class Rooms::MembersController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  def index
    members = @room.users.active.with_attached_avatar.includes(:agent).order(Arel.sql("LOWER(users.name) ASC"), :id).to_a
    lease_states = WorkspacePresenceLease.presence_by_user_id(members.map(&:id))

    render json: {
      members: members.map { |member| member_json(member, lease_states:) }
    }
  end

  private
    def request_authentication
      request.format.json? ? head(:unauthorized) : super
    end

    def member_json(member, lease_states:)
      if member.bot? && member.agent
        agent = member.agent
        live = agent.suspended_at.nil? && agent.last_seen_at.present?

        {
          id: member.id,
          name: member.name,
          avatar_url: fresh_user_avatar_url(member),
          online: live,
          presence: live ? "agent" : "offline",
          status: agent.status_note.presence || agent.status.to_s.humanize
        }
      else
        presence = member.effective_presence(lease_states[member.id] || :offline)

        {
          id: member.id,
          name: member.name,
          avatar_url: fresh_user_avatar_url(member),
          online: presence != :offline,
          presence: presence.to_s,
          status: member.custom_status_display
        }
      end
    end
end
