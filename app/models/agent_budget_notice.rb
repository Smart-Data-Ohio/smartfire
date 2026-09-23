class AgentBudgetNotice < ApplicationRecord
  # One row per agent, cap, and day, recording that the agent hit a daily
  # budget cap. The row sources the owner's activity inbox item, so the
  # owner gets one item per cap per day however many requests overflow.
  CAPS = %w[ messages board_posts external_actions ].freeze
  CAP_LABELS = {
    "messages" => "messages",
    "board_posts" => "board posts",
    "external_actions" => "external actions"
  }.freeze

  belongs_to :agent
  has_many :activity_items, as: :source, dependent: :destroy, inverse_of: :source

  validates :cap, inclusion: { in: CAPS }
  validates :day, presence: true
  validates :agent_id, uniqueness: { scope: %i[ cap day ] }

  # Records today's hit for the cap, fanning out the inbox item only when
  # this call created the row. The unique index admits exactly one row per
  # agent, cap, and day, so concurrent overflows still notify once.
  def self.record_for!(agent, cap, day: Date.current)
    notice = create_or_find_by!(agent: agent, cap: cap.to_s, day: day)

    if notice.previously_new_record?
      notice.fan_out_inbox_items!
    end

    notice
  end

  def fan_out_inbox_items!
    recipients.each do |recipient|
      ActivityItems::Recorder.record!(recipient: recipient, source: self,
        event_type: "agent_budget_exceeded", skip_source_check: true)
    end
  end

  # The owner, or every administrator when no owner is recorded (the same
  # audience that decides the agent's approvals).
  def recipients
    if agent.owner
      [ agent.owner ]
    else
      User.active.without_bots.where(role: :administrator).to_a
    end
  end

  def activity_recipient_ids
    recipients.map(&:id)
  end

  def cap_label
    CAP_LABELS.fetch(cap, cap)
  end

  def budget_limit
    agent.public_send(Agents::Budgets.limit_column(cap))
  end
end
