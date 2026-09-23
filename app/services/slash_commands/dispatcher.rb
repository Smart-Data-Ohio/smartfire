module SlashCommands
  # Parses composer text starting with "/" and runs the matching command.
  # Built-ins come from the Registry; anything else is looked up among
  # the room's registered agent commands and delivered to the owning
  # agent as a slash_command event. Unknown commands answer a helpful
  # ephemeral error instead of posting, so a typo never lands in the
  # room as a message.
  class Dispatcher
    COMMAND_PATTERN = %r{\A/(?<name>[a-zA-Z][a-zA-Z0-9_-]*)(?:\s+(?<args>.*))?\z}m
    SHRUG = "¯\\_(ツ)_/¯"
    # Mirror of the agent message-delivery rate: one room's humans cannot
    # flood an agent's command webhook past this.
    AGENT_COMMANDS_PER_MINUTE = 20

    class << self
      def command_text?(text)
        text.to_s.strip.match?(COMMAND_PATTERN) && !text.to_s.strip.start_with?("//")
      end

      def dispatch(user:, room:, thread: nil, text:)
        match = text.to_s.strip.match(COMMAND_PATTERN)
        return Registry::Result.error("Type / to see available commands.") unless match

        name = match[:name].downcase
        args = match[:args].to_s.strip
        context = Registry::Context.new(user, room, thread, args, Rails.application.routes.url_helpers)

        if (command = Registry.lookup(name))
          return Registry::Result.error("“/#{name}” is only available in the channel, not in threads.") unless command.permission.call(user, room, thread)

          Handlers.send(command.handler, context)
        elsif (registration = AgentSlashCommand.find_by(room_id: room.id, name: name))
          invoke_agent_command(registration, context)
        else
          Registry::Result.error(unknown_command_message(user, room, thread, name))
        end
      rescue ChannelThread::LockedError
        Registry::Result.error("This thread is locked.")
      end

      private
        def unknown_command_message(user, room, thread, name)
          names = Registry.available_for(user, room, thread: thread).map { |command| "/#{command.name}" }
          names += AgentSlashCommand.where(room_id: room.id).ordered.pluck(:name).map { |n| "/#{n}" }
          "Unknown command “/#{name}”. Available: #{names.join(", ")}."
        end

        def invoke_agent_command(registration, context)
          agent = registration.agent
          room = context.room

          unless agent.active? &&
              Membership.exists?(user_id: agent.user_id, room_id: room.id) &&
              agent.can?(:post_messages, room)
            return Registry::Result.error("“/#{registration.name}” is no longer available.")
          end

          if agent_rate_limited?(agent, room)
            return Registry::Result.error("“/#{registration.name}” is receiving too many invocations right now. Try again in a minute.")
          end

          event = agent.agent_events.create!(
            event_type: "slash_command",
            room: room,
            actor: context.user,
            outcome: "delivered",
            chain_id: SecureRandom.uuid,
            metadata: { "command" => registration.name, "arguments" => context.args, "thread_id" => context.thread&.id }.compact
          )
          Agent::Delivery.deliver_command_webhook(event)

          Registry::Result.ephemeral("Sent to #{agent.user.name}")
        end

        def agent_rate_limited?(agent, room)
          agent.agent_events.where(event_type: "slash_command", room_id: room.id)
            .where("created_at >= ?", 1.minute.ago)
            .where(outcome: %w[ pending delivered acknowledged ])
            .count >= AGENT_COMMANDS_PER_MINUTE
        end
    end
  end
end
