class Rooms::MembersController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  def index
    members = @room.users.active.with_attached_avatar.includes(:agent, :meeting_cache).order(Arel.sql("LOWER(users.name) ASC"), :id).to_a
    lease_states = WorkspacePresenceLease.presence_by_user_id(members.map(&:id))
    # Per viewer, computed live on every request: this JSON is never
    # cached or etagged across viewers, so one viewer's stars cannot
    # leak into another's panel.
    starred_ids = Current.user.starred_ids_among(members.map(&:id))
    no_store_response!

    render json: {
      members: members.map { |member| member_json(member, lease_states:, starred_ids:) }
    }
  end

  private
    def request_authentication
      request.format.json? ? head(:unauthorized) : super
    end

    def member_json(member, lease_states:, starred_ids:)
      starred = starred_ids.include?(member.id)

      if member.bot? && member.agent
        agent = member.agent
        live = agent.suspended_at.nil? && agent.last_seen_at.present?

        {
          id: member.id,
          name: member.name,
          avatar_url: fresh_user_avatar_url(member),
          bot: member.bot?,
          online: live,
          presence: live ? "agent" : "offline",
          status: agent.working_presence_text.presence || agent.status_note.presence || agent.status.to_s.humanize,
          starred:
        }
      else
        presence = member.effective_presence(lease_states[member.id] || :offline)

        {
          id: member.id,
          name: member.name,
          avatar_url: fresh_user_avatar_url(member),
          bot: member.bot?,
          online: presence != :offline,
          presence: presence.to_s,
          status: member.status_text_display,
          starred:
        }
      end
    end
end
