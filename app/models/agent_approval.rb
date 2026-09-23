class AgentApproval < ApplicationRecord
  STATUSES = %w[ pending approved denied cancelled expired ].freeze
  DECISIONS = %w[ approved denied ].freeze
  ACTION_FORMAT = /\A[a-z0-9_.-]+\z/
  PAYLOAD_MAX_BYTES = 4.kilobytes
  DEFAULT_TTL = 24.hours
  MIN_TTL = 5.minutes
  MAX_TTL = 7.days

  belongs_to :agent
  belongs_to :room, optional: true
  belongs_to :agent_credential, optional: true
  belongs_to :decided_by, class_name: "User", optional: true

  has_many :activity_items, as: :source, dependent: :destroy, inverse_of: :source

  validates :action, presence: true, length: { maximum: 60 }, format: { with: ACTION_FORMAT }
  validates :summary, presence: true, length: { maximum: 500 }
  validates :status, presence: true, inclusion: { in: STATUSES }
  validates :expires_at, presence: true
  validates :decision_note, length: { maximum: 200 }, allow_nil: true
  validates :external_id, uniqueness: { scope: :agent_id }, allow_nil: true
  validate :payload_size_within_limit
  validate :expires_at_within_bounds, on: :create

  before_validation :normalize_external_id, :default_expires_at, on: :create

  after_create_commit :fan_out_inbox_items

  # Expires pending approvals whose time ran out and marks their inbox
  # items handled, so deciders' unread badges drop. Runs lazily from the
  # activity inbox like Huddle::InvitationResolver and stays idempotent:
  # only unhandled approval items past expiry are touched.
  def self.resolve_overdue!(user: nil)
    scope = ActivityItem.where(event_type: "agent_approval_request", handled_at: nil)
    scope = scope.where(user_id: user.id) if user
    scope.preload(:source).find_each do |item|
      approval = item.source if item.source_type == AgentApproval.polymorphic_name
      approval&.expire_if_due!
    end
  end

  # Lazy expiry: a stored pending row past its deadline reads as expired.
  # Every read path uses this; writers persist via #expire_if_due!.
  def effective_status
    if status == "pending" && expires_at.present? && expires_at <= Time.current
      "expired"
    else
      status
    end
  end

  def pending_effective?
    effective_status == "pending"
  end

  def expired_effective?
    effective_status == "expired"
  end

  # Persists a lazily-expired pending row. Returns true when it expired here.
  def expire_if_due!
    return false unless status == "pending" && expires_at.present? && expires_at <= Time.current

    transaction do
      update!(status: "expired")
      mark_inbox_items_handled!
    end
    true
  end

  # Human decision. Raises ActiveRecord::RecordInvalid when the request is
  # already decided, cancelled, or expired. The pending check, the status
  # write, and the ledger row share one locked transaction so two
  # concurrent decisions cannot both pass the guard; the webhook is
  # enqueued only after every open transaction commits, so a slow webhook
  # host never holds the database write lock nor the human's request.
  def decide!(decision:, by:, note: nil)
    decision = decision.to_s
    raise ArgumentError, "Unknown decision: #{decision}" unless DECISIONS.include?(decision)

    expire_if_due!
    event = with_lock do
      ensure_pending!
      update!(
        status: decision,
        decided_by: by,
        decided_at: Time.current,
        decision_note: note.presence
      )
      mark_inbox_items_handled!
      record_decision_event!
    end

    if decision == "approved" && github_action?
      Github::PerformAgentActionJob.perform_later(id)
    end

    if decision == "approved" && fizzy_action?
      Fizzy::PerformAgentActionJob.perform_later(id)
    end

    if event.webhook_pending?
      ActiveRecord.after_all_transactions_commit do
        Agent::EventWebhookJob.perform_later(event.id, event.webhook_attempts.to_i)
      end
    end
    self
  end

  # Agent cancellation. Raises ActiveRecord::RecordInvalid once decided or
  # expired. Unlike a human decision, this appends no ledger event: the
  # agent already knows it cancelled.
  def cancel_by_agent!
    expire_if_due!
    with_lock do
      ensure_pending!
      update!(status: "cancelled")
      mark_inbox_items_handled!
    end

    self
  end

  def github_action?
    action.to_s.start_with?("github.")
  end

  # True when account is the GitHub connection recorded when this github.*
  # action was requested: the same connection row, still linked to the same
  # GitHub login. A request that recorded none never matches.
  def github_identity_matches?(account)
    account.present? && github_account_id.present? && github_login.present? &&
      account.id == github_account_id && account.github_login.to_s.casecmp?(github_login)
  end

  # The agent's current GitHub connection, if it still is the one recorded
  # on this request.
  def github_identity_current?
    github_identity_matches?(Github::AgentIdentity.resolve(agent))
  end

  def fizzy_action?
    action.to_s.start_with?("fizzy.")
  end

  # True when account is the Fizzy connection recorded when this fizzy.*
  # action was requested: the same connection row, still linked to the
  # same Fizzy user. A request that recorded none never matches.
  def fizzy_identity_matches?(account)
    account.present? && fizzy_connected_account_id.present? && fizzy_user_id.present? &&
      account.id == fizzy_connected_account_id && account.fizzy_user_id.to_s == fizzy_user_id.to_s
  end

  # The agent owner's current Fizzy connection, if it still is the one
  # recorded on this request.
  def fizzy_identity_current?
    fizzy_identity_matches?(agent&.owner&.fizzy_connected_account)
  end

  def decidable_by?(user)
    return false unless user&.active? && !user.bot?
    return false unless agent&.user&.active?

    user.administrator? || agent.owner_id == user.id
  end

  # Approving a GitHub or Fizzy write action makes the agent act on an
  # external service, so only a current administrator may approve one,
  # even as the agent's owner; owners may still deny. Other actions
  # follow decidable_by?.
  def approvable_by?(user)
    decidable_by?(user) && (!(github_action? || fizzy_action?) || user.administrator?)
  end

  def deciders
    admins = User.active.without_bots.where(role: :administrator).to_a
    owner = agent&.owner
    ([ owner ] + admins).compact.uniq.filter { |user| user.active? && !user.bot? }
  end

  # Contract for ActivityItems::Recorder-style checks; creation fans out
  # directly so every decider gets exactly one item.
  def activity_recipient_ids
    deciders.map(&:id)
  end

  private
    # Runs inside the caller's locked transaction, on the reloaded row. A
    # deadline that passed since the caller's own expire_if_due! reads as
    # expired without being persisted here, because raising would roll the
    # write back anyway; the next reader persists it.
    def ensure_pending!
      return if pending_effective?

      errors.add(:base, already_settled_message)
      raise ActiveRecord::RecordInvalid.new(self)
    end

    def normalize_external_id
      self.external_id = external_id.presence
    end

    def default_expires_at
      self.expires_at ||= DEFAULT_TTL.from_now
    end

    def payload_size_within_limit
      return if payload.nil?

      if payload.to_s.bytesize > PAYLOAD_MAX_BYTES
        errors.add(:payload, "is too large (maximum is 4 KB)")
      end
    end

    # The agent may request 5 minutes to 7 days. A one-minute tolerance on
    # both ends keeps exact-boundary requests from flaking as time passes
    # between assignment and validation.
    def expires_at_within_bounds
      return if expires_at.blank?

      now = Time.current
      if expires_at < (MIN_TTL.from_now(now) - 1.minute)
        errors.add(:expires_at, "must be at least 5 minutes from now")
      elsif expires_at > (MAX_TTL.from_now(now) + 1.minute)
        errors.add(:expires_at, "must be within 7 days from now")
      end
    end

    def already_settled_message
      if status == "expired"
        "Request has expired"
      else
        "Request is already #{status}"
      end
    end

    def fan_out_inbox_items
      deciders.each do |recipient|
        next unless recipient.inbox_preferences.agent_approvals

        ActivityItem.create_or_find_by!(user: recipient, source: self) do |item|
          item.event_type = "agent_approval_request"
        end
      end
    end

    def mark_inbox_items_handled!
      now = Time.current
      activity_items.where(handled_at: nil).find_each do |item|
        item.update!(read_at: item.read_at || now, handled_at: now)
      end
    end

    def record_decision_event!
      pending_webhook = agent.user.webhook.present?
      event = agent.agent_events.create!(
        event_type: "approval_decided",
        room: room,
        actor: decided_by,
        outcome: "delivered",
        agent_approval_id: id,
        webhook_status: pending_webhook ? "pending" : "none",
        webhook_next_attempt_at: (Time.current if pending_webhook),
        metadata: {
          "approval_id" => id,
          "status" => status,
          "decided_by" => decided_by&.name,
          "note" => decision_note
        }
      )
      event
    end
end
