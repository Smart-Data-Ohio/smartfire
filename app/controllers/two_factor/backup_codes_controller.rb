module TwoFactor
  # Regenerates the signed-in user's backup codes from the profile,
  # invalidating the old set, and shows the new set once.
  class BackupCodesController < ApplicationController
    def create
      no_store_response!
      credential = Current.user.two_factor_credential
      return redirect_to two_factor_setup_url unless credential&.enabled?

      @backup_codes = TwoFactorBackupCode.regenerate_set!(credential)
      AuditLog.record!(action: "two_factor.backup_codes.regenerate", target: Current.user)
      @continue_url = user_profile_url
      render :show
    end
  end
end
