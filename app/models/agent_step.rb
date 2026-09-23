class AgentStep < ApplicationRecord
  # Structured progress entries an agent attaches to its own message or to
  # a work thread it owns. Rendered as a collapsible step list, like a CI
  # log (see agent_steps/_steps). All content renders escaped.
  STATUSES = %w[ pending running done failed ].freeze
  NAME_LIMIT = 120
  SUMMARY_LIMIT = 1000
  MAX_PER_PARENT = 50

  belongs_to :agent
  belongs_to :message, optional: true
  belongs_to :channel_thread, class_name: "ChannelThread", foreign_key: :channel_thread_id, optional: true

  validates :name, presence: true, length: { maximum: NAME_LIMIT }
  validates :status, inclusion: { in: STATUSES }
  validates :input_summary, :output_summary, length: { maximum: SUMMARY_LIMIT }, allow_blank: true
  validates :duration_ms, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validate :exactly_one_parent
  validate :parent_belongs_to_agent
  validate :parent_step_limit, on: :create

  before_create :assign_position

  scope :ordered, -> { order(:position, :id) }

  def parent
    message || channel_thread
  end

  private
    def exactly_one_parent
      if message_id.present? == channel_thread_id.present?
        errors.add(:base, "Step must belong to exactly one message or thread")
      end
    end

    # A step rides on the agent's own message or a work thread the agent
    # owns. Anything else is rejected here as well as by the services.
    def parent_belongs_to_agent
      if message_id.present? && message && message.creator_id != agent.user_id
        errors.add(:message, "must be the agent's own message")
      end

      if channel_thread_id.present? && channel_thread &&
          !(channel_thread.work? && channel_thread.work_owner_id == agent.user_id)
        errors.add(:channel_thread, "must be work the agent owns")
      end
    end

    def parent_step_limit
      count = if message_id.present?
        self.class.where(message_id: message_id).count
      elsif channel_thread_id.present?
        self.class.where(channel_thread_id: channel_thread_id).count
      else
        0
      end

      if count >= MAX_PER_PARENT
        errors.add(:base, "Steps are limited to #{MAX_PER_PARENT} per message or thread")
      end
    end

    def assign_position
      siblings = if message_id.present?
        self.class.where(message_id: message_id)
      else
        self.class.where(channel_thread_id: channel_thread_id)
      end

      self.position = (siblings.maximum(:position) || -1) + 1
    end
end
