require "test_helper"

class Users::ProfilesTwoFactorTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:david)
  end

  test "profile shows the 2FA section with devices and revoke buttons" do
    credential = enroll_two_factor!(@user)
    device, _token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "TestBrowser/1.0", ip_address: "1.2.3.4")
    expired, _token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "OldBrowser/1.0", ip_address: "5.6.7.8")
    expired.update!(expires_at: 1.minute.ago)
    sign_in @user

    get user_profile_url

    assert_response :success
    assert_select "h2", text: "Two-step sign-in"
    assert_select "##{dom_id(expired)}", count: 0
    assert_select "form[action='#{two_factor_backup_codes_path}']", count: 1
    assert_select "form[action='#{two_factor_setup_path}'] input[name='_method'][value='delete']", count: 1
    assert_select "##{dom_id(device)}", text: /TestBrowser/
    assert_select "##{dom_id(device)} form[action='#{two_factor_remembered_device_path(device)}']", count: 1
    assert credential.reload.enabled?
  end

  test "profile points unenrolled users at setup" do
    sign_in @user

    get user_profile_url

    assert_response :success
    assert_select "a[href='#{two_factor_setup_path}']", text: "Set up two-step sign-in"
  end

  test "changing the password revokes all remembered devices" do
    enroll_two_factor!(@user)
    TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
    sign_in @user

    assert_difference -> { @user.two_factor_remembered_devices.count }, -1 do
      patch user_profile_url, params: { user: { password: "new-secret-123456" } }
    end

    assert_redirected_to user_profile_url
    assert AuditLog.exists?(action: "user.password.change", target_id: @user.id)
  end

  test "updating the name keeps remembered devices" do
    enroll_two_factor!(@user)
    TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
    sign_in @user

    assert_no_difference -> { @user.two_factor_remembered_devices.count } do
      patch user_profile_url, params: { user: { name: "David H" } }
    end
  end
end
