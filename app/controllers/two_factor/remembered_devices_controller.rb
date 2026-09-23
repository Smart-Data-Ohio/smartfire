module TwoFactor
  # Revokes one remembered device from the profile. The cookie becomes
  # useless once its server-side row is gone.
  class RememberedDevicesController < ApplicationController
    def destroy
      Current.user.two_factor_remembered_devices.find_by(id: params[:id])&.destroy!
      redirect_to user_profile_url, notice: "Device forgotten. It will ask for a code at next sign-in."
    end
  end
end
