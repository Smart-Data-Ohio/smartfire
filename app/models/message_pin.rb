class MessagePin < ApplicationRecord
  MAX_PER_ROOM = 50

  class CapReachedError < StandardError; end

  belongs_to :message
  belongs_to :room
  belongs_to :pinner, class_name: "User"

  validates :message_id, uniqueness: true
  validate :message_must_belong_to_room

  scope :ordered, -> { order(created_at: :desc, id: :desc) }

  class << self
    # Pins a message for a room member (or the member's agent). Raises
    # CapReachedError past MAX_PER_ROOM and RecordInvalid when the message
    # is already pinned. Posts the pin note and broadcasts inside the same
    # transaction; the count check runs under SQLite's immediate write
    # lock, so two concurrent pins cannot both pass the cap.
    def pin!(message:, pinner:)
      transaction do
        room = message.room
        raise CapReachedError, "This channel already has #{MAX_PER_ROOM} pinned messages" if room.message_pins.count >= MAX_PER_ROOM

        pin = create!(message:, room:, pinner:)
        pin.post_pin_note!
        pin.broadcast_pin_change
        pin
      end
    end

    def pinned?(message)
      exists?(message_id: message.id)
    end
  end

  def unpin!
    transaction do
      destroy!
      broadcast_pin_change
    end
  end

  def broadcast_pin_change
    Turbo::StreamsChannel.broadcast_replace_to(
      room, :messages,
      target: ActionView::RecordIdentifier.dom_id(message, :pin_badge),
      partial: "messages/pin_badge",
      locals: { message: },
      attributes: { maintain_scroll: true }
    )
    Turbo::StreamsChannel.broadcast_replace_to(
      room, :messages,
      target: ActionView::RecordIdentifier.dom_id(room, :pins_count),
      partial: "rooms/pins/count",
      locals: { room: },
      attributes: { maintain_scroll: true }
    )
    Turbo::StreamsChannel.broadcast_replace_to(
      room, :messages,
      target: ActionView::RecordIdentifier.dom_id(room, :pins_list),
      partial: "rooms/pins/list",
      locals: { room: },
      attributes: { maintain_scroll: true }
    )
  end

  # A one-line channel note through the same path event announcements
  # use, so every client appends it over the room messages stream.
  def post_pin_note!
    room.root_messages.create_with_attachment!(
      creator: pinner, markdown_source: "📌 pinned a message: [jump to message](#{pin_permalink})"
    ).tap(&:broadcast_create)
  end

  private
    def message_must_belong_to_room
      if message && room_id != message.room_id
        errors.add :room, "must be the message's room"
      end
    end

    def pin_permalink
      helpers = Rails.application.routes.url_helpers
      if message.thread_message?
        helpers.room_url(room, thread: message.thread_id, message_id: message.id, only_path: true)
      else
        helpers.room_at_message_path(room, message)
      end
    end
end
