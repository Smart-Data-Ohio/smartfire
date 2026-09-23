module Agents
  # Agent slash-command registration for the REST endpoints and the MCP
  # register_slash_command/unregister_slash_command tools. Owns the
  # membership rule (404 for rooms outside the agent's membership) and
  # the post_messages gate in the room, so both surfaces enforce the
  # same access. Registering is idempotent per agent: re-registering
  # the agent's own name updates its description. Names are unique per
  # room across agents, so invocation is never ambiguous; a name held
  # by another agent answers 422, and unregistering a name the agent
  # does not own answers 404 either way.
  class SlashCommands
    def self.register(agent:, room_id:, name:, description: nil)
      room, denial = find_authorized_room(agent, room_id)
      return denial if denial

      normalized = name.to_s.strip.downcase
      command = AgentSlashCommand.find_by(room_id: room.id, name: normalized)

      if command && command.agent_id != agent.id
        return ServiceResult.fail("“/#{normalized}” is already registered in this room", status: :unprocessable_entity)
      end

      command ||= agent.agent_slash_commands.build(room: room)
      command.name = normalized
      command.description = description.to_s.strip.presence

      begin
        command.save!
      rescue ActiveRecord::RecordInvalid => error
        return ServiceResult.fail(error.record.errors.full_messages.to_sentence)
      end

      ServiceResult.ok(command_payload(command), status: :created)
    end

    def self.unregister(agent:, room_id:, name:)
      room, denial = find_authorized_room(agent, room_id)
      return denial if denial

      command = AgentSlashCommand.find_by(room_id: room.id, name: name.to_s.strip.downcase, agent_id: agent.id)
      return ServiceResult.fail("Command not found", status: :not_found) unless command

      command.destroy!

      ServiceResult.ok({ unregistered: true, name: command.name, room_id: room.id })
    end

    def self.find_authorized_room(agent, room_id)
      room = agent.user.rooms.find_by(id: room_id)
      return [ nil, ServiceResult.fail("Room not found", status: :not_found) ] unless room

      unless agent.can?(:post_messages, room)
        return [ nil, ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden) ]
      end

      [ room, nil ]
    end
    private_class_method :find_authorized_room

    def self.command_payload(command)
      { name: command.name, description: command.description, room_id: command.room_id, agent_id: command.agent_id }
    end
    private_class_method :command_payload
  end
end
