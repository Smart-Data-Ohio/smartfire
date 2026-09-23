# Single-use backup codes for two-step sign-in, stored as SHA-256
# digests only (like bot keys and agent credentials). Plaintext exists
# only in the response that generated it.
class TwoFactorBackupCode < ApplicationRecord
  CODE_LENGTH = 10

  belongs_to :two_factor_credential, class_name: "TwoFactorCredential"

  validates :code_digest, presence: true, uniqueness: true

  scope :unused, -> { where(used_at: nil) }

  def self.generate_code
    SecureRandom.alphanumeric(CODE_LENGTH).downcase
  end

  def self.digest(code)
    Digest::SHA256.hexdigest(normalize(code))
  end

  def self.normalize(code)
    code.to_s.gsub(/[\s-]+/, "").downcase
  end

  # Replaces the credential's full set, invalidating every old code, and
  # returns the new plaintext codes for the one-time display.
  def self.regenerate_set!(credential)
    codes = Array.new(TwoFactorCredential::BACKUP_CODE_COUNT) { generate_code }

    transaction do
      where(two_factor_credential_id: credential.id).delete_all
      codes.each do |code|
        create!(two_factor_credential: credential, code_digest: digest(code))
      end
    end

    codes
  end

  # Atomically consumes one unused matching code. The conditional UPDATE
  # both checks and claims, so two concurrent requests cannot spend the
  # same code. Returns true iff a code was consumed.
  def self.consume!(credential, code)
    return false if normalize(code).blank?

    where(two_factor_credential_id: credential.id, code_digest: digest(code), used_at: nil)
      .update_all(used_at: Time.current, updated_at: Time.current) == 1
  end

  def used?
    used_at.present?
  end
end
