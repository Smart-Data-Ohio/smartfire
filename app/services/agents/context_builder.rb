module Agents
  # Conversation context for GET /agents/context and the MCP get_context
  # tool. Given the triggering message (or a thread outright), returns
  # the message, its thread summary and root message, the last N messages
  # of the same conversation ending at the trigger, and the room. Enforces
  # room membership and the read_messages grant. The presenter renders
  # message payloads; window authors carry agent/human flags.
  class ContextBuilder
    DEFAULT_LIMIT = 30
    MAX_LIMIT = 100

    def self.build(agent:, message_id: nil, thread_id: nil, limit: nil, presenter:)
      if message_id.blank? && thread_id.blank?
        return ServiceResult.fail("message_id or thread_id is required")
      end

      parsed_limit = (limit.presence || DEFAULT_LIMIT).to_i
      parsed_limit = DEFAULT_LIMIT if parsed_limit < 1
      parsed_limit = [ parsed_limit, MAX_LIMIT ].min

      message = nil
      thread = nil

      if message_id.present?
        message = Message.find_by(id: message_id)
        return ServiceResult.fail("Message not found", status: :not_found) unless message

        thread = message.thread
      else
        thread = ChannelThread.find_by(id: thread_id)
        return ServiceResult.fail("Thread not found", status: :not_found) unless thread
      end

      room = (thread || message).room
      room = agent.user.rooms.find_by(id: room.id)
      if room.nil?
        return ServiceResult.fail(message_id.present? ? "Message not found" : "Thread not found", status: :not_found)
      end
      unless agent.can?(:read_messages, room)
        return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
      end

      # After authorization: an unauthorized caller gets the same 404 or
      # 403 whatever thread_id it passes, never a mismatch oracle.
      if message_id.present? && thread_id.present? && thread_id.to_i != message.thread_id.to_i
        return ServiceResult.fail("Message is not in the given thread")
      end

      scope = thread ? thread.messages : room.root_messages
      scope = scope.where("messages.id <= ?", message.id) if message
      window = scope.with_payload_details.reorder(id: :desc).limit(parsed_limit).to_a.reverse

      root_message = thread&.parent_message
      creators = (window.map(&:creator) + [ message&.creator, root_message&.creator ]).compact.uniq(&:id)
      flags_by_creator_id = creators.to_h do |creator|
        [ creator.id, { agent: creator.bot?, human: !creator.bot? } ]
      end

      payload = presenter.caching_thread_payloads do
        {
          message: message ? decorate_author(presenter.message_payload(message), flags_by_creator_id) : nil,
          thread: thread ? thread_summary(thread) : nil,
          root_message: root_message ? decorate_author(presenter.message_payload(root_message), flags_by_creator_id) : nil,
          messages: window.map { |record| decorate_author(presenter.message_payload(record), flags_by_creator_id) },
          authors: creators.map { |creator| { id: creator.id, name: creator.name }.merge(flags_by_creator_id[creator.id]) },
          room: { id: room.id, name: room.name, purpose: nil }
        }
      end

      ServiceResult.ok(payload)
    end

    def self.thread_summary(thread)
      {
        id: thread.id,
        name: thread.name,
        status: thread.status,
        room_id: thread.room_id,
        parent_message_id: thread.parent_message_id
      }
    end

    def self.decorate_author(payload, human_by_creator_id)
      return payload if payload.nil?

      creator = payload[:creator]
      if creator && (flags = human_by_creator_id[creator[:id]])
        payload = payload.merge(creator: creator.merge(flags))
      end

      payload
    end
    private_class_method :thread_summary, :decorate_author
  end
end
