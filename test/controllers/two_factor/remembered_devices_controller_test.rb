require "test_helper"

class TwoFactor::RememberedDevicesControllerTest < ActionDispatch::IntegrationTest
  include GoogleSignInTestHelper
  include GoogleCalendarTestHelper

  setup do
    @user = users(:david)
    @credential = enroll_two_factor!(@user)
    @device, _token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
  end

  test "revoke deletes the device" do
    sign_in @user

    assert_difference -> { @user.two_factor_remembered_devices.count }, -1 do
      delete two_factor_remembered_device_url(@device), params: { reauth: totp_code_for(@credential) }
    end

    assert_redirected_to user_profile_url
  end

  test "revoke accepts the password" do
    sign_in @user

    assert_difference -> { @user.two_factor_remembered_devices.count }, -1 do
      delete two_factor_remembered_device_url(@device), params: { reauth: "secret123456" }
    end

    assert_redirected_to user_profile_url
  end

  test "revoke refuses without re-authentication" do
    sign_in @user

    assert_no_difference -> { @user.two_factor_remembered_devices.count } do
      delete two_factor_remembered_device_url(@device)
    end

    assert_redirected_to user_profile_url
  end

  test "revoke refuses a wrong code" do
    sign_in @user

    assert_no_difference -> { @user.two_factor_remembered_devices.count } do
      delete two_factor_remembered_device_url(@device), params: { reauth: "000000" }
    end

    assert_redirected_to user_profile_url
  end

  test "revoke cannot touch another user's device" do
    other, _token = TwoFactorRememberedDevice.create_for!(users(:jason), user_agent: "Browser", ip_address: "1.2.3.4")
    sign_in @user

    assert_no_difference -> { users(:jason).two_factor_remembered_devices.count } do
      delete two_factor_remembered_device_url(other), params: { reauth: totp_code_for(@credential) }
    end

    assert_redirected_to user_profile_url
  end

  test "revoke of an unknown device still lands on the profile" do
    sign_in @user

    delete two_factor_remembered_device_url(123456789), params: { reauth: totp_code_for(@credential) }

    assert_redirected_to user_profile_url
  end

  test "revoke redirects visitors to sign in" do
    delete two_factor_remembered_device_url(@device)

    assert_redirected_to new_session_url
  end

  test "destroy_all revokes every device and audits" do
    TwoFactorRememberedDevice.create_for!(@user, user_agent: "Other", ip_address: "5.6.7.8")
    sign_in @user

    assert_difference -> { @user.two_factor_remembered_devices.count }, -2 do
      delete two_factor_remembered_devices_url, params: { reauth: totp_code_for(@credential) }
    end

    assert_redirected_to user_profile_url
    assert AuditLog.exists?(action: "two_factor.devices.revoke_all", target_id: @user.id)
  end

  test "destroy_all refuses without re-authentication" do
    sign_in @user

    assert_no_difference -> { @user.two_factor_remembered_devices.count } do
      delete two_factor_remembered_devices_url
    end

    assert_redirected_to user_profile_url
    assert_not AuditLog.exists?(action: "two_factor.devices.revoke_all", target_id: @user.id)
  end

  test "destroy_all accepts the password" do
    sign_in @user

    assert_difference -> { @user.two_factor_remembered_devices.count }, -1 do
      delete two_factor_remembered_devices_url, params: { reauth: "secret123456" }
    end

    assert_redirected_to user_profile_url
  end

  test "destroy_all accepts a completed Google re-auth" do
    identity = link_google_identity!(@user)
    sign_in @user

    state = start_google_reauth
    complete_google_sign_in(state:, sub: identity.subject, email: identity.email)
    assert_redirected_to user_profile_url

    assert_difference -> { @user.two_factor_remembered_devices.count }, -1 do
      delete two_factor_remembered_devices_url
    end

    assert_redirected_to user_profile_url
  end

  test "revoke is rate limited" do
    sign_in @user

    with_rate_limit_store do
      10.times { delete two_factor_remembered_device_url(@device), params: { reauth: "000000" } }
      assert_redirected_to user_profile_url

      delete two_factor_remembered_device_url(@device), params: { reauth: "secret123456" }
      assert_redirected_to user_profile_url
      assert_equal "Too many attempts. Try again in a few minutes.", flash[:alert]
      assert_equal 1, @user.two_factor_remembered_devices.count
    end
  end
end
