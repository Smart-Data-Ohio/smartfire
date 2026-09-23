module Agents
  # Shared work handoff for the REST agent handoff API and the MCP
  # handoff_work tool. Owns the ownership rule (the agent owns the thread,
  # still belongs to its room, and still holds read_messages there), the
  # manage_threads gate for the sender, and the receiver checks (an
  # active agent member holding post_messages, manage_threads, and
  # read_messages), so both surfaces enforce the same access. The human
  # handoff controller
  # authorizes its sender itself and shares WorkHandoff.receiver_error
  # for the receiver.
  class WorkHandoffs
    def self.create(agent:, id:, receiver_agent_id:, summary:, links: nil, open_questions: nil)
      owned = WorkThreads.show(agent: agent, id: id)
      return owned unless owned.ok?

      thread = owned.payload

      unless agent.can?(:manage_threads, thread.room)
        return ServiceResult.fail("Forbidden: agent lacks manage_threads capability", status: :forbidden)
      end

      receiver = Agent.find_by(id: receiver_agent_id)
      if (error = WorkHandoff.receiver_error(thread, receiver))
        return ServiceResult.fail(error)
      end

      begin
        handoff = thread.hand_off!(
          sender: agent.user,
          receiver_agent: receiver,
          summary: summary,
          links: links || [],
          open_questions: open_questions || []
        )
      rescue ActiveRecord::RecordNotFound
        return ServiceResult.fail("Work not found", status: :not_found)
      rescue ActiveRecord::RecordInvalid => error
        return ServiceResult.fail(error.record.errors.full_messages.to_sentence)
      end

      ServiceResult.ok({ thread: thread.reload, handoff: handoff }, status: :created)
    end
  end
end
