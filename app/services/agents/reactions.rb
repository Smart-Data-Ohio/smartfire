module Agents
  # Reactions for the MCP react tool. Idempotent: repeating the same
  # content returns the existing boost instead of stacking (or, like the
  # human toggle, removing) it, so retried tool calls are safe. Enforces
  # room membership and the react grant.
  class Reactions
    def self.react(agent:, message_id:, content:)
      message = Message.find_by(id: message_id)
      return ServiceResult.fail("Message not found", status: :not_found) unless message

      room = agent.user.rooms.find_by(id: message.room_id)
      return ServiceResult.fail("Message not found", status: :not_found) unless room
      unless agent.can?(:react, room)
        return ServiceResult.fail("Forbidden: agent lacks react capability", status: :forbidden)
      end

      resolved = Boost.resolve_content(content)
      if resolved.blank?
        return ServiceResult.fail("Reaction content can't be blank")
      end

      if (existing = message.boosts.where(booster: agent.user, content: resolved).first)
        return ServiceResult.ok({ id: existing.id, message_id: message.id, content: existing.content, created: false })
      end

      boost = message.boosts.create!(booster: agent.user, content: resolved)
      message.broadcast_reactions_replace

      ServiceResult.ok({ id: boost.id, message_id: message.id, content: boost.content, created: true })
    end
  end
end
