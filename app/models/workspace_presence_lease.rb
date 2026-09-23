class WorkspacePresenceLease < ApplicationRecord
  TTL = 90.seconds
  IDLE_AFTER = 10.minutes

  belongs_to :session
  belongs_to :user

  scope :unexpired, -> { where(expires_at: Time.current..) }

  validates :connection_id, presence: true, uniqueness: true
  validates :expires_at, presence: true

  class << self
    # Establishing and reading never prune: both run on hot paths (every
    # tab connect and every 60 s presence poll), and a DELETE takes the
    # SQLite write lock. The periodic sweep below removes stale rows.
    def establish(user:, session:)
      return unless identity_valid?(user:, session:)

      create!(
        connection_id: SecureRandom.uuid,
        expires_at: TTL.from_now,
        last_active_at: Time.current,
        session: session,
        user: user
      )
    end

    def online_user_ids(user_ids)
      presence_by_user_id(user_ids).keys
    end

    # Live lease state per user id: :online when any lease saw activity
    # within IDLE_AFTER, :idle when leases exist but all went quiet. Ids
    # without a lease are absent; readers treat them as :offline. One
    # query for the whole set. A nil activity stamp (a lease established
    # before this column existed) counts as active; heartbeats refresh it.
    def presence_by_user_id(user_ids)
      cutoff = IDLE_AFTER.ago
      live_leases_for(user_ids).pluck(:user_id, :last_active_at)
        .group_by(&:first)
        .transform_values do |rows|
          rows.any? { |(_, active_at)| active_at.nil? || active_at >= cutoff } ? :online : :idle
        end
    end

    def live_leases_for(user_ids)
      unexpired.joins(:session, :user)
        .merge(User.active)
        .where(user_id: user_ids)
        .where("sessions.user_id = workspace_presence_leases.user_id")
        .distinct
    end

    def identity_valid?(user:, session:)
      user && session &&
        User.active.exists?(id: user.id) &&
        Session.exists?(id: session.id, user_id: user.id)
    end

    # The periodic sweep calls this; reads never do (see above).
    def prune(limit: 100)
      stale_ids = where(<<~SQL.squish, Time.current).limit(limit).pluck(:id)
        expires_at < ? OR NOT EXISTS (
          SELECT 1 FROM sessions
          WHERE sessions.id = workspace_presence_leases.session_id
            AND sessions.user_id = workspace_presence_leases.user_id
        )
      SQL

      where(id: stale_ids).delete_all
    end
  end

  # The heartbeat always extends the lease; only a heartbeat that
  # reports recent input extends activity, so a quiet tab idles out.
  def refresh(active: false)
    if self.class.identity_valid?(user:, session:)
      now = Time.current
      attributes = active ? { expires_at: TTL.from_now, last_active_at: now } : { expires_at: TTL.from_now }
      update_columns(attributes)
    else
      delete
      false
    end
  end
end
