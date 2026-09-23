require "test_helper"

class TwoFactorRememberedDeviceTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
  end

  test "find_valid returns the device and stamps its use" do
    device, token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
    device.update_columns(last_used_at: 1.day.ago, updated_at: 1.day.ago)

    assert_equal device, TwoFactorRememberedDevice.find_valid(token, @user)
    assert_in_delta Time.current, device.reload.last_used_at, 5.seconds
  end

  test "only the digest is stored" do
    _device, token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")

    assert_equal Digest::SHA256.hexdigest(token), @user.two_factor_remembered_devices.sole.token_digest
  end

  test "find_valid rejects blank, unknown, and foreign tokens" do
    device, token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")

    assert_nil TwoFactorRememberedDevice.find_valid(nil, @user)
    assert_nil TwoFactorRememberedDevice.find_valid("", @user)
    assert_nil TwoFactorRememberedDevice.find_valid("0" * 64, @user)
    assert_nil TwoFactorRememberedDevice.find_valid(token, users(:jason))
    assert_nil TwoFactorRememberedDevice.find_valid(token, nil)
    assert device.reload.present?
  end

  test "find_valid rejects expired devices" do
    device, token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")
    device.update!(expires_at: 1.minute.ago)

    assert_nil TwoFactorRememberedDevice.find_valid(token, @user)
  end

  test "devices expire after 30 days" do
    device, _token = TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")

    assert_in_delta 30.days.from_now, device.expires_at, 5.seconds
  end

  test "reset_two_factor! clears the credential, codes, and devices" do
    enroll_two_factor!(@user)
    TwoFactorBackupCode.regenerate_set!(@user.two_factor_credential)
    TwoFactorRememberedDevice.create_for!(@user, user_agent: "Browser", ip_address: "1.2.3.4")

    @user.reset_two_factor!

    assert_not @user.reload.two_factor_enabled?
    assert_equal 0, TwoFactorBackupCode.count
    assert_equal 0, @user.two_factor_remembered_devices.count
  end
end
