require "test_helper"

class TwoFactor::RememberedDevicesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:david)
    enroll_two_factor!(@user)
    @device, _token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
  end

  test "revoke deletes the device" do
    sign_in @user

    assert_difference -> { @user.two_factor_remembered_devices.count }, -1 do
      delete two_factor_remembered_device_url(@device)
    end

    assert_redirected_to user_profile_url
  end

  test "revoke cannot touch another user's device" do
    other, _token = TwoFactorRememberedDevice.create_for!(users(:jason), user_agent: "Browser", ip_address: "1.2.3.4")
    sign_in @user

    assert_no_difference -> { users(:jason).two_factor_remembered_devices.count } do
      delete two_factor_remembered_device_url(other)
    end

    assert_redirected_to user_profile_url
  end

  test "revoke of an unknown device still lands on the profile" do
    sign_in @user

    delete two_factor_remembered_device_url(123456789)

    assert_redirected_to user_profile_url
  end

  test "revoke redirects visitors to sign in" do
    delete two_factor_remembered_device_url(@device)

    assert_redirected_to new_session_url
  end
end
