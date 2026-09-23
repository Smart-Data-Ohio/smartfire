# Enforced two-step sign-in for human users. Bots and agent tokens
# authenticate with keys and are exempt (see TwoFactorEnforcement).
module User::TwoFactor
  extend ActiveSupport::Concern

  included do
    has_one :two_factor_credential, dependent: :destroy
    has_many :two_factor_remembered_devices, dependent: :delete_all
  end

  def two_factor_enabled?
    two_factor_credential&.enabled? || false
  end

  def requires_two_factor?
    active? && !bot?
  end

  def revoke_two_factor_remembered_devices!
    two_factor_remembered_devices.delete_all
  end

  # Destroys the credential (and its backup codes) plus every remembered
  # device, returning the user to the must-enroll state.
  def reset_two_factor!
    transaction do
      two_factor_credential&.destroy!
      revoke_two_factor_remembered_devices!
    end
  end
end
