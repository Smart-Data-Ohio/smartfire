# TOTP (RFC 6238) credential for enforced two-step sign-in. One row per
# human user at most; bots never have one. The secret is encrypted at
# rest by Active Record encryption. A row with no confirmed_at is a
# pending enrollment, not yet usable for sign-in.
class TwoFactorCredential < ApplicationRecord
  ISSUER = "Smartfire"
  # ±1 TOTP step (30 seconds each way) of clock drift.
  DRIFT_SECONDS = 30
  BACKUP_CODE_COUNT = 10
  # After this many consecutive failed challenge codes the challenge
  # locks, for the 1st, 2nd, then 3rd-and-later durations below. Any
  # success resets both the run and the escalation.
  FAILURES_BEFORE_LOCKOUT = 5
  LOCKOUT_DURATIONS = [ 1.minute, 5.minutes, 15.minutes ].freeze

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

  # Confirms a pending enrollment with a code against the caller's
  # session-bound pending secret (never the stored secret: until confirmed
  # the row is shared by every session that opened setup). On success the
  # pending secret becomes the credential, the used step is stamped so the
  # enrollment code cannot also sign in, and the pending secret is spent.
  def confirm_with_setup_secret!(setup_secret, code)
    normalized = code.to_s.gsub(/\s+/, "")
    return false if normalized.blank?

    with_lock do
      matched_at = ROTP::TOTP.new(setup_secret.secret).verify(normalized,
        drift_ahead: DRIFT_SECONDS, drift_behind: DRIFT_SECONDS, after: last_totp_at)
      return false if matched_at.nil?

      update!(secret: setup_secret.secret, confirmed_at: Time.current, last_totp_at: matched_at)
      setup_secret.destroy!
      true
    end
  rescue ArgumentError
    false
  end

  def locked_out?
    locked_until.present? && locked_until > Time.current
  end

  # Records a failed challenge code. The 5th consecutive failure starts a
  # lockout at the next escalation level and resets the run, so each
  # lockout takes 5 fresh failures. Returns :locked when a lockout just
  # started, :failed otherwise. Runs under the row lock so concurrent
  # failures count exactly once each.
  def register_challenge_failure!
    with_lock do
      return :failed if locked_out?

      failures = consecutive_failures + 1
      if failures >= FAILURES_BEFORE_LOCKOUT
        level = lockout_count + 1
        update!(consecutive_failures: 0, lockout_count: level,
          locked_until: lockout_duration_for(level).from_now)
        :locked
      else
        update!(consecutive_failures: failures)
        :failed
      end
    end
  end

  # A verified code clears the failure run, any live lockout, and the
  # escalation level.
  def register_challenge_success!
    update!(consecutive_failures: 0, lockout_count: 0, locked_until: nil)
  end

  def lockout_duration_for(level)
    LOCKOUT_DURATIONS.fetch(level - 1, LOCKOUT_DURATIONS.last)
  end
end
