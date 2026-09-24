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
    # takes_arguments defaults to true (picking inserts "/name " and waits);
    # pass false for a command that takes no arguments so the composer runs
    # it immediately when picked. An omitted flag keeps the stored value on
    # re-registration.
    def self.register(agent:, room_id:, name:, description: nil, takes_arguments: nil)
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
      command.takes_arguments = ActiveModel::Type::Boolean.new.cast(takes_arguments) unless takes_arguments.nil?

      begin
        command.save!
      rescue ActiveRecord::RecordInvalid => error
        return ServiceResult.fail(error.record.errors.full_messages.to_sentence)
      rescue ActiveRecord::RecordNotUnique
        return adopt_race_winner(agent, room, normalized, description, takes_arguments)
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
      { name: command.name, description: command.description, room_id: command.room_id, agent_id: command.agent_id, takes_arguments: command.takes_arguments }
    end
    private_class_method :command_payload

    # A concurrent registration won the room/name index between the
    # lookup and the save. Adopt the outcome instead of erroring: a
    # name that landed on this agent completes the re-registration,
    # anything else held reports taken like the lookup path does.
    def self.adopt_race_winner(agent, room, name, description, takes_arguments)
      winner = AgentSlashCommand.find_by(room_id: room.id, name: name)

      if winner&.agent_id == agent.id
        attributes = { description: description.to_s.strip.presence }
        attributes[:takes_arguments] = ActiveModel::Type::Boolean.new.cast(takes_arguments) unless takes_arguments.nil?
        if winner.update(attributes)
          ServiceResult.ok(command_payload(winner), status: :created)
        else
          ServiceResult.fail(winner.errors.full_messages.to_sentence)
        end
      else
        ServiceResult.fail("“/#{name}” is already registered in this room", status: :unprocessable_entity)
      end
    end
    private_class_method :adopt_race_winner
  end
end
