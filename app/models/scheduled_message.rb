class ScheduledMessage < ApplicationRecord
  # A claim older than this is presumed orphaned (its runner died between
  # claiming and posting) and becomes claimable again.
  STALE_CLAIM_AFTER = 5.minutes

  belongs_to :user
  belongs_to :room
  belongs_to :thread, class_name: "ChannelThread", optional: true
  belongs_to :reply_to_message, class_name: "Message", optional: true
  belongs_to :sent_message, class_name: "Message", optional: true

  has_many :activity_items, as: :source, dependent: :destroy

  validates :markdown_source, presence: true, length: { maximum: Message::Markdown::SOURCE_LIMIT }
  validate :send_at_must_be_future, if: :will_save_change_to_send_at?
  validate :conversation_links

  scope :ordered, -> { order(:send_at, :id) }
  scope :pending, -> { where(sent_at: nil, dropped_at: nil) }
  scope :past, -> { where.not(sent_at: nil).or(where.not(dropped_at: nil)).order(send_at: :desc, id: :desc) }

  class << self
    # The author's rows for the Scheduled view, with sent history newest
    # first. Room access is re-checked per row at render time (see
    # #sendable?), like saved items: losing access hides the row without
    # deleting it.
    def owned_by(user)
      where(user_id: user.id)
    end

    def due(now = Time.current)
      pending.where(send_at: ..now)
    end
  end

  def pending?
    sent_at.nil? && dropped_at.nil?
  end

  def sent?
    sent_at.present?
  end

  def dropped?
    dropped_at.present?
  end

  def conversation
    thread || room
  end

  # True while the author can still post in the room: active human,
  # live room, current membership, and (for thread rows) a live thread
  # in that room. The dispatcher and send-now re-check this at send
  # time; the index uses it to hide stranded rows.
  def sendable?
    return false unless user&.active? && !user.bot?
    return false unless room && !room.deleted?
    return false unless room.memberships.exists?(user_id: user_id)
    return false if thread && thread.room_id != room_id

    true
  end

  private
    def send_at_must_be_future
      if send_at.present? && send_at <= Time.current
        errors.add :send_at, "must be in the future"
      end
    end

    def conversation_links
      if thread && thread.room_id != room_id
        errors.add :thread, "must belong to the scheduled room"
      end

      return unless reply_to_message

      source = reply_to_message
      same_stream = if thread_id.nil?
        source.thread_id.nil?
      else
        source.thread_id == thread_id
      end

      errors.add :reply_to_message, "must be in the same conversation" unless source.room_id == room_id && same_stream
    end
end
