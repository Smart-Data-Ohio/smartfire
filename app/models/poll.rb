class Poll < ApplicationRecord
  MIN_OPTIONS = 2
  MAX_OPTIONS = 10

  belongs_to :message
  has_one :room, through: :message

  has_many :poll_options, -> { order(:position, :id) }, dependent: :destroy
  has_many :poll_votes, dependent: :destroy

  validates :message_id, uniqueness: true
  validate :closes_at_must_be_future, if: :will_save_change_to_closes_at?

  scope :open, -> { where(closed_at: nil).where("closes_at IS NULL OR closes_at > ?", Time.current) }

  class << self
    def normalize_labels(labels)
      Array(labels).map { |label| label.to_s.strip }.reject(&:blank?)
    end

    # Attaches a poll to an already-saved message with the given option
    # labels. Shared by the human and agent creation paths so both
    # validate identically. Raises ActiveRecord::RecordInvalid.
    # Streaming messages never carry polls: the card renders the final
    # question, which does not exist until the stream finalizes.
    def create_for_message!(message:, labels:, multiple: false, anonymous: false, closes_at: nil)
      if message.streaming?
        message.errors.add :base, "A streaming message cannot carry a poll"
        raise ActiveRecord::RecordInvalid.new(message)
      end

      labels = normalize_labels(labels)
      unless labels.size.between?(MIN_OPTIONS, MAX_OPTIONS)
        message.errors.add :base, "Poll needs between #{MIN_OPTIONS} and #{MAX_OPTIONS} options"
        raise ActiveRecord::RecordInvalid.new(message)
      end

      transaction do
        poll = message.create_poll!(multiple:, anonymous:, closes_at:)
        labels.each_with_index do |label, index|
          poll.poll_options.create!(label:, position: index)
        end
        poll
      end
    end

    # Stamps due polls closed and refreshes their cards. Runs from the
    # periodic runner; a poll's closed display must converge without
    # waiting for the next vote, and the stamp busts the message
    # fragment cache that renders the card.
    def close_due!(now: Time.current)
      where(closed_at: nil).where("closes_at IS NOT NULL AND closes_at <= ?", now).find_each do |poll|
        poll.close!(now:)
      rescue => error
        Rails.logger.error "Poll close failed for poll #{poll.id}: #{error.class}: #{error.message}"
      end
    end
  end

  def question
    message.plain_text_body
  end

  def single?
    !multiple?
  end

  def closed?(now: Time.current)
    closed_at.present? || (closes_at.present? && closes_at <= now)
  end

  def open?(now: Time.current)
    !closed?(now:)
  end

  def close!(now: Time.current)
    claimed = self.class.where(id: id, closed_at: nil).update_all(closed_at: now, updated_at: now) == 1
    return false unless claimed

    reload
    broadcast_card_replace
    true
  end

  # Replaces the voter's ballot with the given option ids (empty retracts).
  # Raises ActiveRecord::RecordInvalid when the poll is closed or an
  # option does not belong to it. Touches the poll so the message
  # fragment cache key moves with every vote.
  def cast_vote!(voter, option_ids)
    ids = Array(option_ids).map { |id| id.to_i }.uniq
    options = poll_options.to_a

    if ids.any? && (ids - options.map(&:id)).any?
      errors.add :base, "That option is not part of this poll"
      raise ActiveRecord::RecordInvalid.new(self)
    end
    if single? && ids.size > 1
      errors.add :base, "This poll allows only one option"
      raise ActiveRecord::RecordInvalid.new(self)
    end

    transaction do
      lock!
      if closed?
        errors.add :base, "This poll is closed"
        raise ActiveRecord::RecordInvalid.new(self)
      end

      poll_votes.where(user_id: voter.id).delete_all
      ids.each do |option_id|
        poll_votes.create!(poll_option_id: option_id, user: voter)
      end
      touch
    end

    broadcast_card_replace
    true
  end

  def total_votes
    poll_votes.size
  end

  # One payload for the card, the vote JSON response, and the agent API.
  # Anonymous polls carry counts only; the voter rows still exist (one
  # ballot per person, changeable until close) but never render.
  def results_payload(viewer: nil)
    votes = poll_votes.to_a
    viewer_ids = viewer ? votes.select { |vote| vote.user_id == viewer.id }.map(&:poll_option_id).to_set : Set.new

    {
      id: id,
      message_id: message_id,
      room_id: message.room_id,
      question: question,
      multiple: multiple?,
      anonymous: anonymous?,
      closes_at: closes_at,
      closed_at: closed_at,
      closed: closed?,
      total_votes: votes.size,
      options: poll_options.map do |option|
        option_votes = votes.select { |vote| vote.poll_option_id == option.id }
        entry = { id: option.id, label: option.label, votes: option_votes.size, voted: viewer_ids.include?(option.id) }
        entry[:voters] = option_votes.filter_map { |vote| vote.user&.name }.sort unless anonymous?
        entry
      end
    }
  end

  def broadcast_card_replace
    message.broadcast_replace_to message.message_stream_target, :messages,
      target: ActionView::RecordIdentifier.dom_id(self, :card),
      partial: "polls/poll", locals: { poll: self }, attributes: { maintain_scroll: true }
  end

  private
    def closes_at_must_be_future
      if closes_at.present? && closes_at <= Time.current
        errors.add :closes_at, "must be in the future"
      end
    end
end
