class AgentCredential < ApplicationRecord
  # `last_used_at`/`last_used_ip` are stamped at most this often per
  # credential, like Agent#touch_last_seen!.
  RECORD_USE_THROTTLE = 1.minute

  belongs_to :agent
  belongs_to :created_by, class_name: "User"

  validates :name, presence: true
  validates :token_digest, presence: true, uniqueness: true
  validates :token_last_four, presence: true

  scope :not_revoked, -> { where(revoked_at: nil) }
  scope :not_expired, -> { where("expires_at IS NULL OR expires_at > ?", Time.current) }
  scope :active, -> { not_revoked.not_expired }

  class << self
    def generate_secret
      SecureRandom.hex(32)
    end

    def digest(secret)
      Digest::SHA256.hexdigest(secret.to_s)
    end

    def create_with_secret!(agent:, name:, created_by:, expires_at: nil)
      secret = generate_secret

      credential = create!(
        agent: agent,
        name: name,
        created_by: created_by,
        expires_at: expires_at,
        token_digest: digest(secret),
        # Display identifier only: derived from the digest so no secret substring is stored.
        token_last_four: digest(secret)[0, 4]
      )

      [ credential, secret ]
    end

    def authenticate(secret)
      return if secret.blank?

      credential = find_by(token_digest: digest(secret.strip))
      credential if credential&.active?
    end
  end

  def revoked?
    revoked_at.present?
  end

  def expired?
    expires_at.present? && expires_at <= Time.current
  end

  def active?
    !revoked? && !expired?
  end

  def revoke!
    update!(revoked_at: Time.current) unless revoked?
  end

  # Stamps use without callbacks or validations, at most once per minute
  # per credential. The throttle check and the write are one conditional
  # UPDATE so concurrent requests cannot both observe an expired value and
  # write. An IP change inside the throttle window is recorded with the
  # next stamp, not immediately.
  def record_use!(ip = nil)
    now = Time.current
    written = AgentCredential.where(id: id)
      .where("last_used_at IS NULL OR last_used_at <= ?", now - RECORD_USE_THROTTLE)
      .update_all(last_used_at: now, last_used_ip: ip, updated_at: now)

    if written.positive?
      write_attribute(:last_used_at, now)
      clear_attribute_change(:last_used_at)
      write_attribute(:last_used_ip, ip)
      clear_attribute_change(:last_used_ip)
      write_attribute(:updated_at, now)
      clear_attribute_change(:updated_at)
    end
  end
end
