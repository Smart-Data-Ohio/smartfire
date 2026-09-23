# One board's SLA rule for one work status, configured by the board's
# creator or an administrator. A post sitting in the status past
# nudge_after_minutes nudges its owner (see
# BoardAutomations::SlaDispatcher for the recipient rule); past
# escalate_after_minutes it escalates to the board's creator. Each stage
# fires once per status crossing. See docs/board-automations.md.
class BoardSlaRule < ApplicationRecord
  MAX_MINUTES = 43_200 # 30 days

  belongs_to :room

  validates :work_status, presence: true, inclusion: { in: ChannelThread::WORK_STATUSES },
    uniqueness: { scope: :room_id }
  validates :nudge_after_minutes, presence: true,
    numericality: { only_integer: true, greater_than: 0, less_than_or_equal_to: MAX_MINUTES }
  validates :escalate_after_minutes, presence: true,
    numericality: { only_integer: true, greater_than: 0, less_than_or_equal_to: MAX_MINUTES }
  validate :escalation_must_follow_nudge
  validate :room_must_be_board

  private
    def escalation_must_follow_nudge
      return if nudge_after_minutes.blank? || escalate_after_minutes.blank?

      if escalate_after_minutes <= nudge_after_minutes
        errors.add(:escalate_after_minutes, "must be after the nudge threshold")
      end
    end

    def room_must_be_board
      errors.add(:room, "must be a board") if room && !room.board?
    end
end
