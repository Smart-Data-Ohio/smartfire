class Agent < ApplicationRecord
  # Capabilities a legacy agent (one with zero grant rows ever created) keeps
  # in rooms it belongs to. Once any grant has ever existed, only active
  # grants count; revoking the last grant removes access.
  LEGACY_CAPABILITIES = %w[ read_messages post_messages react ].freeze

  encrypts :webhook_signing_secret

  # Self-reported status vocabulary, set only by the agent itself through
  # PATCH /agents/me. `waiting` means waiting on a human. Suspension is
  # separate and still comes from `suspended_at`.
  STATUSES = %w[ idle working waiting failed ].freeze

  # `last_seen_at` is touched at most this often per agent.
  LAST_SEEN_THROTTLE = 1.minute

  # Working presence ("Thinking…", "Running tests…"), shown next to the
  # agent's name in the room member list. Expires after the TTL unless
  # refreshed, and clears when the agent's stream finalizes.
  WORKING_PRESENCE_LIMIT = 140
  WORKING_PRESENCE_TTL = 5.minutes

  belongs_to :user
  belongs_to :owner, class_name: "User", optional: true

  has_many :agent_credentials, dependent: :destroy
  has_many :agent_grants, dependent: :destroy
  has_many :agent_slash_commands, dependent: :destroy
  has_many :agent_events, dependent: :destroy
  has_many :agent_approvals, dependent: :destroy

  enum :kind, { personal: "personal", workspace: "workspace" }, default: :personal

  validates :user_id, uniqueness: true
  validates :owner_id, presence: true, if: :personal?
  validates :owner_id, presence: true, on: :create, if: :workspace?

  validates :status, inclusion: { in: STATUSES }
  validates :description, length: { maximum: 500 }, allow_nil: true
  validates :status_note, length: { maximum: 200 }, allow_nil: true
  validates :working_presence, length: { maximum: WORKING_PRESENCE_LIMIT }, allow_nil: true
  validates :daily_message_cap, :daily_board_post_cap, :daily_external_action_cap,
    numericality: { only_integer: true, greater_than: 0 }, allow_nil: true

  before_update -> { AgentGrant.revoke_for_agent!(self) },
    if: -> { will_save_change_to_suspended_at? && suspended_at.present? }
  before_update :stamp_status_changed_at, if: :will_save_change_to_status?
  after_update_commit :broadcast_status_change, if: :saved_status_change?

  # Agents for the directory: active first, then suspended or otherwise
  # inactive, each group name-sorted. Deactivated users are excluded.
  def self.for_directory
    includes(:user, :owner).joins(:user)
      .where.not(users: { status: "deactivated" })
      .to_a
      .sort_by { |agent| [ agent.active? ? 0 : 1, agent.user.name.downcase ] }
  end

  def active?
    suspended_at.nil? && user&.active?
  end

  def suspended?
    suspended_at.present?
  end

  # Suspension fans out from deactivation, bans, and bot removal, so the
  # audit row lives here rather than at each call site. The actor defaults
  # to whoever triggered the suspension through the current request.
  def suspend!
    return if suspended?

    update!(suspended_at: Time.current)
    AuditLog.record!(action: "agent.suspend", target: self)
  end

  # The kill switch: suspends the agent (revoking every grant, which also
  # blocks approved-but-unexecuted external actions at perform time),
  # cancels every still-pending approval request, clears working
  # presence, and records `agent.kill_switch`. Returns the number of
  # approvals cancelled.
  def kill_switch!
    suspend!

    cancelled = 0
    transaction do
      agent_approvals.where(status: "pending").find_each do |approval|
        approval.expire_if_due!
        if approval.pending_effective?
          approval.cancel_by_agent!
          cancelled += 1
        end
      end
      clear_working_presence! if working_presence.present?
    end

    AuditLog.record!(action: "agent.kill_switch", target: self,
      changes: { pending_approvals_cancelled: cancelled })
    cancelled
  end

  # The working presence text, or nil when none is set or the TTL ran
  # out. Expired rows read as cleared everywhere; no sweep rewrites them.
  def working_presence_text(now: Time.current)
    return nil if working_presence.blank?
    return nil if working_presence_expires_at.present? && working_presence_expires_at <= now

    working_presence
  end

  # Stages a working presence (or clears it when blank) on this instance,
  # so PATCH /agents/me saves it together with the status fields.
  def assign_working_presence(text)
    if text.to_s.strip.blank?
      self.working_presence = nil
      self.working_presence_expires_at = nil
    else
      self.working_presence = text.to_s.strip
      self.working_presence_expires_at = WORKING_PRESENCE_TTL.from_now
    end
  end

  def set_working_presence!(text)
    assign_working_presence(text)
    save!
  end

  def clear_working_presence!
    return unless working_presence.present?

    update!(working_presence: nil, working_presence_expires_at: nil)
  end

  def kind_description
    return "no owner recorded" if owner.nil?

    if personal?
      "Personal agent of #{owner.name}"
    else
      "Workspace agent, managed by #{owner.name}"
    end
  end

  # Compact summary of active grants, e.g. "post_messages in 3 rooms,
  # read_messages workspace-wide". Capabilities with no active grant are
  # omitted. Reads the database on every call; no caching.
  def grants_summary
    return "legacy access (no grants recorded)" if legacy_capabilities?

    summaries = AgentGrant::CAPABILITIES.sort.filter_map do |capability|
      grants = agent_grants.active.where(capability: capability).to_a
      next if grants.empty?

      if grants.any?(&:workspace_wide?)
        "#{capability} workspace-wide"
      else
        "#{capability} in #{grants.size} #{"room".pluralize(grants.size)}"
      end
    end

    summaries.presence&.join(", ") || "no active grants"
  end

  # Compact 24-hour activity summary drawn from the delivery ledger.
  # `posted` rows carry outcome "delivered", so delivered counts
  # deliverable types only and each row lands in exactly one bucket.
  def activity_summary
    scope = agent_events.where("agent_events.created_at >= ?", 24.hours.ago)
    delivered = scope.deliverable.where(outcome: "delivered").count
    acknowledged = scope.where(outcome: "acknowledged").count
    posted = scope.where(event_type: "posted").count
    suppressed = scope.where(outcome: "suppressed").count

    "#{delivered} delivered, #{acknowledged} acknowledged, #{posted} posted, #{suppressed} suppressed"
  end

  # Records agent activity without callbacks, validations, or broadcasts.
  # Throttled to at most once per minute per agent; the throttle check and
  # the write are one conditional UPDATE so concurrent requests cannot both
  # observe an expired value and write.
  def touch_last_seen!
    now = Time.current
    written = Agent.where(id: id)
      .where("last_seen_at IS NULL OR last_seen_at <= ?", now - LAST_SEEN_THROTTLE)
      .update_all(last_seen_at: now)

    if written.positive?
      write_attribute(:last_seen_at, now)
      clear_attribute_change(:last_seen_at)
    end
  end

  # True when no agent_grants rows exist for this agent at all, revoked or
  # not. Reads the database on every call; no caching.
  def legacy_capabilities?
    AgentGrant.where(agent_id: id).none?
  end

  # Room-scoped capability check. Reads the database on every call; no
  # caching. Room membership is checked separately by the controllers. A
  # soft-deleted room grants nothing, however the grant row reads.
  def can?(capability, room = nil)
    return false unless active?
    return false if room.is_a?(Room) && room.deleted?

    capability = capability.to_s
    return false unless AgentGrant::CAPABILITIES.include?(capability)

    if legacy_capabilities?
      return LEGACY_CAPABILITIES.include?(capability)
    end

    room_id = case room
    when Room then room.id
    when Integer then room
    when nil then nil
    else room.try(:id)
    end
    scope = AgentGrant.active.where(agent_id: id, capability: capability)

    if room_id
      scope.where(room_id: [ room_id, nil ]).exists?
    else
      scope.where(room_id: nil).exists?
    end
  end

  # The HMAC secret signing this agent's webhook deliveries, generated
  # lazily on first delivery so older rows need no backfill. Shown to
  # admins and the owner on the bot edit page, never logged. Generated
  # under the agent's row lock with a fresh read, so concurrent first
  # deliveries cannot mint competing secrets and invalidate each
  # other's signatures.
  def ensure_webhook_signing_secret!
    return webhook_signing_secret if webhook_signing_secret.present?

    with_lock do
      reload
      return webhook_signing_secret if webhook_signing_secret.present?

      update!(webhook_signing_secret: self.class.generate_webhook_signing_secret)
      webhook_signing_secret
    end
  end

  def reset_webhook_signing_secret!
    with_lock do
      update!(webhook_signing_secret: self.class.generate_webhook_signing_secret)
      webhook_signing_secret
    end
  end

  def self.generate_webhook_signing_secret
    SecureRandom.hex(32)
  end

  # True when the agent holds the capability in any room or workspace-wide.
  # Used by endpoints without a room context (event polling). Reads the
  # database on every call; no caching.
  def has_capability_anywhere?(capability)
    return false unless active?

    capability = capability.to_s
    return false unless AgentGrant::CAPABILITIES.include?(capability)

    return LEGACY_CAPABILITIES.include?(capability) if legacy_capabilities?

    AgentGrant.active.where(agent_id: id, capability: capability).exists?
  end

  private
    def stamp_status_changed_at
      self.status_changed_at = Time.current
    end

    def saved_status_change?
      saved_change_to_status? || saved_change_to_status_note?
    end

    # Replaces the profile status badge and the directory row over the
    # agents stream. The rendered badge and row carry no credentials or
    # grants, so every signed-in human may subscribe.
    def broadcast_status_change
      Turbo::StreamsChannel.broadcast_replace_to(
        AgentsChannel::STREAM_NAME,
        target: ActionView::RecordIdentifier.dom_id(self, :status_badge),
        partial: "agents/status_badge",
        locals: { agent: self }
      )
      Turbo::StreamsChannel.broadcast_replace_to(
        AgentsChannel::STREAM_NAME,
        target: ActionView::RecordIdentifier.dom_id(self, :directory_row),
        partial: "agents/directory/agent",
        locals: { agent: self }
      )
    end
end
