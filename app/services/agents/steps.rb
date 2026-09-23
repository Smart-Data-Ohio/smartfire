module Agents
  # Shared agent steps for POST/PATCH /agents/steps and the MCP
  # add_step / update_step tools. An agent attaches structured progress
  # entries (name, status, input/output summaries, duration) to its own
  # message or to a work thread it owns. Message steps need
  # post_messages in the message's room; thread steps need
  # manage_threads there. Both surfaces enforce the same access, and the
  # model caps the count and size per parent.
  class Steps
    # fields carries string keys: message_id or thread_id (exactly one),
    # name, status, input_summary, output_summary, duration_ms.
    def self.create(agent:, fields:)
      message, thread, denial = find_parent(agent, fields)
      return denial if denial

      step = AgentStep.new(
        agent: agent,
        message: message,
        channel_thread: thread,
        name: fields["name"],
        status: fields["status"].presence || "running",
        input_summary: fields["input_summary"].presence,
        output_summary: fields["output_summary"].presence,
        duration_ms: fields["duration_ms"]
      )

      if step.save
        broadcast_parent(step)
        ServiceResult.ok(step, status: :created)
      else
        ServiceResult.fail(step.errors.full_messages.to_sentence,
          payload: { errors: step.errors.to_hash })
      end
    end

    # Each field updates only when its key is given. Steps never move
    # between parents.
    def self.update(agent:, id:, fields:)
      step = AgentStep.where(agent_id: agent.id).includes(:message, :channel_thread).find_by(id: id)
      return ServiceResult.fail("Step not found", status: :not_found) unless step

      unless step_accessible?(agent, step)
        return ServiceResult.fail("Forbidden: agent lacks #{required_capability(step)} capability", status: :forbidden)
      end

      step.name = fields["name"] if fields.key?("name")
      step.status = fields["status"] if fields.key?("status")
      step.input_summary = fields["input_summary"] if fields.key?("input_summary")
      step.output_summary = fields["output_summary"] if fields.key?("output_summary")
      step.duration_ms = fields["duration_ms"] if fields.key?("duration_ms")

      if step.save
        broadcast_parent(step)
        ServiceResult.ok(step)
      else
        ServiceResult.fail(step.errors.full_messages.to_sentence,
          payload: { errors: step.errors.to_hash })
      end
    end

    def self.step_payload(step)
      {
        id: step.id,
        message_id: step.message_id,
        thread_id: step.channel_thread_id,
        name: step.name,
        status: step.status,
        input_summary: step.input_summary,
        output_summary: step.output_summary,
        duration_ms: step.duration_ms,
        position: step.position,
        created_at: step.created_at&.utc,
        updated_at: step.updated_at&.utc
      }.compact
    end

    # Authorization first: anything the agent must not see (missing
    # parent, outside its rooms, another author's message, unowned
    # thread) is the same 404, before the grant check.
    def self.find_parent(agent, fields)
      message_id = fields["message_id"].presence
      thread_id = fields["thread_id"].presence

      if message_id.present? == thread_id.present?
        return [ nil, nil, ServiceResult.fail("Exactly one of message_id or thread_id is required") ]
      end

      if message_id
        message = Message.find_by(id: message_id)
        if message.nil? || message.creator_id != agent.user_id ||
            !agent.user.rooms.where(id: message.room_id).exists?
          return [ nil, nil, ServiceResult.fail("Message not found", status: :not_found) ]
        end
        unless agent.can?(:post_messages, message.room_id)
          return [ nil, nil, ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden) ]
        end

        [ message, nil, nil ]
      else
        thread = ChannelThread.find_by(id: thread_id)
        if thread.nil? || thread.work_owner_id != agent.user_id ||
            !agent.user.rooms.where(id: thread.room_id).exists?
          return [ nil, nil, ServiceResult.fail("Work not found", status: :not_found) ]
        end
        unless agent.can?(:manage_threads, thread.room_id)
          return [ nil, nil, ServiceResult.fail("Forbidden: agent lacks manage_threads capability", status: :forbidden) ]
        end

        [ nil, thread, nil ]
      end
    end

    # A step stays writable while its parent still qualifies: the
    # agent's own message in one of its rooms with post_messages, or
    # owned work with manage_threads.
    def self.step_accessible?(agent, step)
      if step.message_id.present?
        message = step.message
        message.present? && message.creator_id == agent.user_id &&
          agent.user.rooms.where(id: message.room_id).exists? &&
          agent.can?(:post_messages, message.room_id)
      else
        thread = step.channel_thread
        thread.present? && thread.work? && thread.work_owner_id == agent.user_id &&
          agent.user.rooms.where(id: thread.room_id).exists? &&
          agent.can?(:manage_threads, thread.room_id)
      end
    end

    def self.required_capability(step)
      step.message_id.present? ? "post_messages" : "manage_threads"
    end

    # Re-renders the parent with its new step list: the whole message
    # (steps render inside it), or the thread's steps container. Runs
    # after the save commits, holding no lock.
    def self.broadcast_parent(step)
      if step.message_id.present? && step.message
        step.message.broadcast_replace_to step.message.conversation, :messages, target: step.message
      elsif step.channel_thread_id.present? && step.channel_thread
        thread = step.channel_thread
        thread.broadcast_replace_to thread, :messages,
          target: ActionView::RecordIdentifier.dom_id(thread, :agent_steps),
          partial: "agent_steps/thread_steps", locals: { thread: thread }
      end
    end
    private_class_method :find_parent, :step_accessible?, :required_capability, :broadcast_parent
  end
end
