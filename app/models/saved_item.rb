class SavedItem < ApplicationRecord
  belongs_to :user
  belongs_to :message

  enum :status, %w[ in_progress done ].index_by(&:itself), default: :in_progress, validate: true

  validates :message_id, uniqueness: { scope: :user_id }
  validate :remind_at_must_be_future, if: :will_save_change_to_remind_at?

  before_save :clear_fired_claim, if: :will_save_change_to_remind_at?

  scope :ordered, -> { order(created_at: :desc, id: :desc) }

  class << self
    # Only items whose room the user can still see. The join re-checks
    # membership at view time, so losing room access hides the item
    # without deleting it; regaining access shows it again.
    def accessible_to(user)
      return none unless user&.active? && !user.bot?

      joins(message: :room)
        .joins("INNER JOIN memberships AS saved_item_memberships ON saved_item_memberships.room_id = messages.room_id AND saved_item_memberships.user_id = #{connection.quote(user.id)}")
        .merge(Room.alive)
        .where(user_id: user.id)
    end

    def due_reminders(now = Time.current)
      where.not(remind_at: nil)
        .where(reminded_at: nil)
        .where(remind_at: ..now)
    end
  end

  def reminder_pending?
    remind_at.present? && reminded_at.nil?
  end

  def reminder_fired?
    reminded_at.present?
  end

  # One ActivityItem per user per message source (unique index), so a
  # firing reminder transitions whatever item already exists for the
  # message, the same way event reminders transition invitations.
  def transition_reminder_item!
    attempts = 0
    begin
      ActivityItem.transaction do
        item = ActivityItem.lock.find_or_initialize_by(user:, source: message)
        item.event_type = "message_reminder"
        item.read_at = nil
        item.handled_at = nil
        item.save!
        item
      end
    rescue ActiveRecord::RecordNotUnique
      attempts += 1
      retry if attempts < 2
      raise
    end
  end

  private
    # A new reminder time re-arms the reminder. Without this, a
    # reminded_at left over from an earlier firing would suppress the
    # new reminder, since due_reminders only fires unclaimed items.
    def clear_fired_claim
      self.reminded_at = nil
    end

    def remind_at_must_be_future
      if remind_at.present? && remind_at <= Time.current
        errors.add :remind_at, "must be in the future"
      end
    end
end
