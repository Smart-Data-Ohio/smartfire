# TOTP (RFC 6238) credential for enforced two-step sign-in. One row per
# human user at most; bots never have one. The secret is encrypted at
# rest by Active Record encryption. A row with no confirmed_at is a
# pending enrollment, not yet usable for sign-in.
class TwoFactorCredential < ApplicationRecord
  ISSUER = "Smartfire"
  # ±1 TOTP step (30 seconds each way) of clock drift.
  DRIFT_SECONDS = 30
  BACKUP_CODE_COUNT = 10

  belongs_to :user
  has_many :backup_codes, class_name: "TwoFactorBackupCode", dependent: :delete_all

  encrypts :secret

  validates :user_id, uniqueness: true
  validates :secret, presence: true

  def self.generate_secret
    ROTP::Base32.random_base32(32)
  end

  def enabled?
    confirmed_at.present?
  end

  def totp
    ROTP::TOTP.new(secret, issuer: ISSUER)
  end

  def provisioning_uri
    totp.provisioning_uri(user.email_address)
  end

  # Manual key as spaced groups, the way authenticator apps accept it.
  def formatted_secret
    secret.to_s.scan(/.{1,4}/).join(" ")
  end

  # Verifies a TOTP code, accepting ±1 step of drift and rejecting the
  # last accepted step and earlier (replay protection). The check and the
  # stamp run under the row lock so two concurrent requests cannot both
  # spend the same code. Returns true on success.
  def verify_code(code)
    normalized = code.to_s.gsub(/\s+/, "")
    return false if normalized.blank?

    with_lock do
      matched_at = totp.verify(normalized,
        drift_ahead: DRIFT_SECONDS, drift_behind: DRIFT_SECONDS, after: last_totp_at)
      return false if matched_at.nil?

      update!(last_totp_at: matched_at)
      true
    end
  rescue ArgumentError
    false
  end

  # Confirms a pending enrollment with a valid code. The used step is
  # stamped (see verify_code), so the enrollment code cannot also sign in.
  def confirm!(code)
    return false unless verify_code(code)

    update!(confirmed_at: Time.current) unless enabled?
    true
  end
end
