class AgentGrant < ApplicationRecord
  CAPABILITIES = %w[ read_messages post_messages react manage_threads external_action fizzy dm_anyone ].freeze

  # Every documented capability is enforced: posting and boosting
  # through the bot/agent endpoints, reading through event polling and
  # delivery, external actions through the approval endpoints, work
  # status updates through the agent work endpoints, Fizzy reads
  # through the agent Fizzy endpoints, and agent-initiated DMs to
  # strangers through the agent DM endpoints. See AgentAuthorization.
  ENFORCED_CAPABILITIES = %w[ read_messages post_messages react manage_threads external_action fizzy dm_anyone ].freeze

  belongs_to :agent
  belongs_to :room, optional: true
  belongs_to :granted_by, class_name: "User"

  scope :active, -> { where(revoked_at: nil) }
  scope :revoked, -> { where.not(revoked_at: nil) }

  validates :capability, presence: true, inclusion: { in: CAPABILITIES }
  validates :room, presence: { message: "must be an existing room" }, if: :room_id?
  validate :no_duplicate_active_grant
  validate :dm_anyone_must_be_workspace_wide

  class << self
    def revoke_for_membership!(membership)
      agent_id = Agent.where(user_id: membership.user_id).pick(:id)
      return unless agent_id

      active.where(agent_id: agent_id, room_id: membership.room_id)
        .update_all(revoked_at: Time.current, updated_at: Time.current)
    end

    def revoke_for_room!(room)
      active.where(room_id: room.id)
        .update_all(revoked_at: Time.current, updated_at: Time.current)
    end

    def revoke_for_agent!(agent)
      active.where(agent_id: agent.id)
        .update_all(revoked_at: Time.current, updated_at: Time.current)
    end

    def revoke_for_user!(user)
      agent_id = Agent.where(user_id: user.id).pick(:id)
      return unless agent_id

      active.where(agent_id: agent_id)
        .update_all(revoked_at: Time.current, updated_at: Time.current)
    end
  end

  def active?
    revoked_at.nil?
  end

  def revoked?
    revoked_at.present?
  end

  def workspace_wide?
    room_id.nil?
  end

  def revoke!
    update!(revoked_at: Time.current) unless revoked?
  end

  private
    # dm_anyone gates DMing strangers, who share no room with the agent by
    # definition, so a room scope would be meaningless. Only active
    # workspace-wide grants count; room-scoped rows are rejected here and
    # ignored by the DM check.
    def dm_anyone_must_be_workspace_wide
      if capability == "dm_anyone" && room_id.present?
        errors.add(:room, "must be blank: dm_anyone is granted workspace-wide only")
      end
    end

    def no_duplicate_active_grant
      return if revoked? || capability.blank?

      scope = self.class.active.where(agent_id: agent_id, capability: capability, room_id: room_id)
      scope = scope.where.not(id: id) if persisted?

      errors.add(:capability, "has already been granted") if scope.exists?
    end
end
