module Agents
  # Room listing for the MCP list_rooms tool: the rooms the agent's user
  # belongs to, name-sorted, with the kind flags agents need to pick the
  # right tool (board posts vs channel messages).
  class Rooms
    def self.list(agent:)
      rooms = agent.user.rooms.ordered.to_a

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
  end
end
