module Agents
  # Agent pinning for POST/DELETE /agents/messages/:id/pin and the MCP
  # pin_message/unpin_message tools. Owns the membership rule (404 for
  # messages outside the agent's rooms) and the post_messages gate in the
  # message's room, so both surfaces enforce the same access. Pinning is
  # idempotent: pinning an already-pinned message succeeds without
  # duplicating; unpinning a message that is not pinned still succeeds.
  class Pins
    def self.pin(agent:, message_id:)
      message, room, denial = find_authorized_message(agent, message_id)
      return denial if denial

      begin
        MessagePin.pin!(message: message, pinner: agent.user)
        ServiceResult.ok(pin_payload(message, room, pinned: true), status: :created)
      rescue ActiveRecord::RecordInvalid
        ServiceResult.ok(pin_payload(message, room, pinned: true))
      rescue MessagePin::CapReachedError => error
        ServiceResult.fail(error.message)
      end
    end

    def self.unpin(agent:, message_id:)
      message, room, denial = find_authorized_message(agent, message_id)
      return denial if denial

      message.message_pins.first&.unpin!

      ServiceResult.ok(pin_payload(message, room, pinned: false))
    end

    def self.find_authorized_message(agent, message_id)
      message = Message.find_by(id: message_id)
      return [ nil, nil, ServiceResult.fail("Message not found", status: :not_found) ] unless message

      room = agent.user.rooms.find_by(id: message.room_id)
      return [ nil, nil, ServiceResult.fail("Message not found", status: :not_found) ] unless room

      unless agent.can?(:post_messages, room)
        return [ nil, nil, ServiceResult.fail("Forbidden: agent lacks post_messages capability", status: :forbidden) ]
      end

      [ message, room, nil ]
    end
    private_class_method :find_authorized_message

    def self.pin_payload(message, room, pinned:)
      { pinned: pinned, message_id: message.id, pin_count: room.message_pins.count }
    end
    private_class_method :pin_payload
  end
end
