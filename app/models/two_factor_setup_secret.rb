# Unconfirmed TOTP secret for the setup page, bound to one session. Setup
# reuses the session's live secret across reloads; a new session gets its
# own secret, so someone
# who opens setup with only the password cannot learn the secret another
# session later confirms: confirming only ever uses this session's pending
# secret. The secret is encrypted at rest; expired rows are pruned by the
# retention job.
class TwoFactorSetupSecret < ApplicationRecord
  TTL = 30.minutes

  belongs_to :session

  encrypts :secret

  validates :secret, presence: true
  validates :session_id, uniqueness: true

  # Issues a fresh secret for the session, replacing any previous one.
  def self.issue_for!(session)
    record = find_or_initialize_by(session_id: session.id)
    record.secret = TwoFactorCredential.generate_secret
    record.expires_at = TTL.from_now
    record.save!
    record
  rescue ActiveRecord::RecordNotUnique
    # A concurrent issue won the unique index; rotate its secret instead.
    find_by!(session_id: session.id).tap do |existing|
      existing.update!(secret: TwoFactorCredential.generate_secret, expires_at: TTL.from_now)
    end
  end

  # The session's live pending secret, if any. Expired rows read as
  # missing: the controller issues a fresh one and the old code stops
  # working.
  def self.valid_for(session)
    record = find_by(session_id: session.id)
    record unless record.nil? || record.expired?
  end

  # Keeps a secret the member is still working with alive while they
  # switch to their authenticator app and back.
  def extend_expiry!
    update!(expires_at: TTL.from_now)
  end

  def expired?
    expires_at <= Time.current
  end
end
