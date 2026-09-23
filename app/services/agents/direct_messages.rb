module Agents
  # Agent-initiated DMs for POST /agents/dms and the MCP open_dm tool.
  # Opens (or creates) the 1:1 DM between the agent's bot user and a human
  # target, then posts the agent's message through the standard posting
  # flow. Allowed when the target is the agent's owner, has previously
  # messaged the agent (a mention, reply, or DM delivery in the agent's
  # ledger), or the agent holds the dm_anyone capability anywhere. This
  # gate replaces the post_messages grant check: a DM room cannot carry
  # grants before it exists.
  class DirectMessages
    def self.open_and_post(agent:, user_id:, attributes: {}, drive_file_ids: :absent)
      target = User.find_by(id: user_id)
      return ServiceResult.fail("User not found", status: :not_found) unless target

      if target.bot?
        return ServiceResult.fail("Cannot open a DM with a bot")
      end
      unless target.active?
        return ServiceResult.fail("Cannot open a DM with an inactive account")
      end

      unless allowed?(agent, target)
        return ServiceResult.fail(
          "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability",
          status: :forbidden
        )
      end

      room = Rooms::Direct.find_or_create_for(User.where(id: [ agent.user_id, target.id ]))
      broadcast_new_room(room) if room.previously_new_record?

      result = Posting.post(agent: agent, room: room, attributes: attributes, drive_file_ids: drive_file_ids)
      return result unless result.ok?

      ServiceResult.ok({ room: room, message: result.payload }, status: :created)
    rescue ActiveRecord::RecordNotFound
      # A reply target outside the DM room. Posting raises the same error
      # the human endpoint rescues into its room_not_found page.
      ServiceResult.fail("Reply target not found", status: :not_found)
    end

    def self.allowed?(agent, target)
      return true if agent.owner_id.present? && agent.owner_id == target.id
      return true if agent.has_capability_anywhere?(:dm_anyone)

      agent.agent_events.where(actor_id: target.id, event_type: AgentEvent::MESSAGE_DELIVERABLE_TYPES).exists?
    end

    # A freshly created DM appears in both sidebars immediately, like a
    # human-created one.
    def self.broadcast_new_room(room)
      room.memberships.each do |membership|
        membership.broadcast_prepend_to membership.user, :rooms, target: :direct_rooms, partial: "users/sidebars/rooms/direct"
      end
    end
    private_class_method :allowed?, :broadcast_new_room
  end
end
