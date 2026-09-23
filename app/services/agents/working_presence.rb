module Agents
  # Shared working presence for PATCH /agents/me and the MCP set_presence
  # tool. The agent reports what it is doing ("Thinking…", "Running
  # tests…"); members see it next to the agent's name in the room member
  # list until it is cleared, a stream finalizes, or the TTL runs out.
  class WorkingPresence
    def self.set(agent:, text:)
      agent.assign_working_presence(text)

      if agent.save
        ServiceResult.ok({ working_presence: agent.working_presence_text })
      else
        ServiceResult.fail(agent.errors.full_messages.to_sentence,
          payload: { errors: agent.errors.to_hash })
      end
    end
  end
end
