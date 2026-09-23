# The idempotency claim behind one SLA nudge, and the inbox source the
# recipient reads it from. The dispatcher inserts this row first
# (unique per thread, status, stage, and status entry time) and notifies
# only when the insert wins, so each stage fires exactly once per
# status crossing even across runner restarts. A later status change
# carries a new entry time, which is a new claim.
class BoardSlaNudge < ApplicationRecord
  STAGES = %w[ nudge escalation ].freeze

  belongs_to :room
  belongs_to :channel_thread
  belongs_to :recipient, class_name: "User"

  validates :work_status, presence: true, inclusion: { in: ChannelThread::WORK_STATUSES }
  validates :stage, presence: true, inclusion: { in: STAGES }
  validates :status_entered_at, presence: true
  validates :status_entered_at, uniqueness: {
    scope: %i[ channel_thread_id work_status stage ],
    message: "already fired for this status crossing"
  }

  has_many :activity_items, as: :source, dependent: :destroy, inverse_of: :source

  # Minutes the post had sat in the status when the claim was written.
  def waited_minutes(now: Time.current)
    ((now - status_entered_at) / 60).floor.clamp(0..)
  end

  # The inbox contract: the dispatcher authorizes the single recipient
  # before claiming, so the recorded recipient is the allowed one.
  def activity_recipient_ids
    [ recipient_id ]
  end
  alias recipient_user_ids activity_recipient_ids
end
