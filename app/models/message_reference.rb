# A Smartfire message permalink (`/rooms/:room_id/@:message_id`) quoted in
# another message. The link renders as a quote card (author, room, excerpt,
# time) for viewers who can access the source message, and as a plain
# "Message in a private room" chip for everyone else. Unlike event links,
# the source may live in any room; access is checked per viewer at render
# time, and same-room quotes render inline because every viewer of the
# quoting message is a member of that room.
class MessageReference < ApplicationRecord
  belongs_to :message
  belongs_to :referenced_message, class_name: "Message"

  validates :referenced_message_id, uniqueness: { scope: :message_id }
  validate :no_self_reference

  private
    def no_self_reference
      if message_id.present? && message_id == referenced_message_id
        errors.add :referenced_message, "can't quote its own message"
      end
    end
end
