module Agents
  # Message history for the MCP read_messages tool. Reads one conversation
  # — a room's root messages or a thread's messages — newest-first page
  # with before/after cursors, returned chronological. Enforces room
  # membership and the read_messages grant like every other agent read
  # path. Returns records; the caller presents them with message_payload.
  class Reading
    DEFAULT_LIMIT = 50
    MAX_LIMIT = 100

    def self.read(agent:, room_id: nil, thread_id: nil, before: nil, after: nil, limit: nil)
      if room_id.present? && thread_id.present?
        return ServiceResult.fail("Pass only one of room_id, thread_id")
      end
      if room_id.blank? && thread_id.blank?
        return ServiceResult.fail("room_id or thread_id is required")
      end

      scope = nil
      if room_id.present?
        room = agent.user.rooms.find_by(id: room_id)
        return ServiceResult.fail("Room not found", status: :not_found) unless room
        unless agent.can?(:read_messages, room)
          return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
        end

        scope = room.root_messages
      else
        thread = ChannelThread.find_by(id: thread_id)
        return ServiceResult.fail("Thread not found", status: :not_found) unless thread

        room = agent.user.rooms.find_by(id: thread.room_id)
        return ServiceResult.fail("Thread not found", status: :not_found) unless room
        unless agent.can?(:read_messages, room)
          return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
        end

        scope = thread.messages
      end

      limit = [ (limit.presence || DEFAULT_LIMIT).to_i, 1 ].max
      limit = [ limit, MAX_LIMIT ].min

      conversation = scope
      if before.present?
        anchor = conversation.find_by(id: before)
        return ServiceResult.fail("Message not found", status: :not_found) unless anchor

        scope = scope.where("messages.id < ?", anchor.id)
      end
      if after.present?
        anchor = conversation.find_by(id: after)
        return ServiceResult.fail("Message not found", status: :not_found) unless anchor

        scope = scope.where("messages.id > ?", anchor.id)
      end

      messages = scope.with_payload_details.order(id: :desc).limit(limit).to_a.reverse

      if messages.any?
        page_ids = messages.map(&:id)
        has_more_before = scope.where("messages.id < ?", page_ids.first).exists?
        has_more_after = scope.where("messages.id > ?", page_ids.last).exists?
      else
        has_more_before = has_more_after = false
      end

      ServiceResult.ok({
        messages: messages,
        before: messages.first&.id,
        after: messages.last&.id,
        has_more_before: has_more_before,
        has_more_after: has_more_after
      })
    end
  end
end
