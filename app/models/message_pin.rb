class MessagePin < ApplicationRecord
  MAX_PER_ROOM = 50
  PIN_NOTE_WINDOW = 10.minutes

  class CapReachedError < StandardError; end

  belongs_to :message
  belongs_to :room
  belongs_to :pinner, class_name: "User"

  validates :message_id, uniqueness: true
  validate :message_must_belong_to_room

  # One registration for both halves: like Message's reference syncs,
  # declaring the same method under after_create_commit and
  # after_destroy_commit would keep only one registration.
  after_commit :broadcast_pin_change, on: %i[ create destroy ]

  scope :ordered, -> { order(created_at: :desc, id: :desc) }

  class << self
    # Pins a message for a room member (or the member's agent). Raises
    # CapReachedError past MAX_PER_ROOM and RecordInvalid when the message
    # is already pinned. The pin and its note commit atomically; the count
    # check runs under SQLite's immediate write lock, so two concurrent
    # pins cannot both pass the cap. Every broadcast fires after commit:
    # the note through the explicit call below, the badge, count, and
    # panel through the commit callbacks (which also cover unpin, message
    # deletion, and user deletion).
    def pin!(message:, pinner:)
      pin = nil
      note = nil
      transaction do
        room = message.room
        raise CapReachedError, "This channel already has #{MAX_PER_ROOM} pinned messages" if room.message_pins.count >= MAX_PER_ROOM

        pin = create!(message:, room:, pinner:)
        note = pin.post_pin_note!
      end
      note&.broadcast_create
      pin
    end

    def pinned?(message)
      exists?(message_id: message.id)
    end
  end

  def unpin!
    destroy!
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

  # A quiet one-line system note in the channel: rendered as a compact
  # centered note (see messages/_system_note), but never marking the room
  # unread, pushing, delivering, recording inbox items, or indexing for
  # search. The note source is deterministic per message, so a pin/unpin
  # toggle storm posts at most one note per message per window. Returns
  # the note, or nil when the window already holds one; the caller
  # broadcasts after commit.
  def post_pin_note!
    source = pin_note_source
    return if room.root_messages.where(system_note: true, markdown_source: source)
      .where(created_at: PIN_NOTE_WINDOW.ago..).exists?

    room.root_messages.create_with_attachment!(
      creator: pinner, markdown_source: source, system_note: true
    )
  end

  private
    # The note text only; the system note presentation owns the icon, the
    # actor's name, and the timestamp.
    def pin_note_source
      "pinned a message: [jump to message](#{pin_permalink})"
    end

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
