module Agents
  # Room listing for the MCP list_rooms tool: the alive rooms the agent's
  # user belongs to where the agent holds at least one granted capability
  # (a room-scoped grant there, or any workspace-wide grant), name-sorted,
  # with the kind flags agents need to pick the right tool (board posts
  # vs channel messages). Legacy agents list every member room.
  class Rooms
    def self.list(agent:)
      rooms = agent.user.rooms.ordered.to_a
      rooms = with_grants(agent, rooms) unless agent.legacy_capabilities?

      ServiceResult.ok(rooms.map do |room|
        {
          id: room.id,
          name: room.name,
          type: room.type.demodulize.underscore,
          board: room.board?,
          direct: room.direct?
        }
      end)
    end

    # Mirrors Agent#can? without naming a capability: an inactive agent
    # holds nothing, and anyone else holds a room through a room-scoped or
    # workspace-wide active grant. One query for the whole listing.
    def self.with_grants(agent, rooms)
      return [] unless agent.active?

      # dm_anyone governs opening DMs, not reading rooms, so it lists nothing.
      granted = agent.agent_grants.active.where.not(capability: "dm_anyone").pluck(:room_id)
      return rooms if granted.include?(nil)

      ids = granted.to_set
      rooms.select { |room| ids.include?(room.id) }
    end
    private_class_method :with_grants
  end
end
