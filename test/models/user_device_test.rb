require "test_helper"

class UserDeviceTest < ActiveSupport::TestCase
  test "the first sign-in from anywhere is first_seen" do
    assert_equal :first_seen, UserDevice.record_sign_in!(user: users(:kevin), device_id: "device-a", user_agent: "UA/1")

    assert_equal %w[ device-a ], users(:kevin).user_devices.pluck(:device_id)
  end

  test "signing in from a known device is known and refreshes the user agent" do
    UserDevice.record_sign_in!(user: users(:kevin), device_id: "device-a", user_agent: "UA/1")

    assert_equal :known, UserDevice.record_sign_in!(user: users(:kevin), device_id: "device-a", user_agent: "UA/2")

    assert_equal "UA/2", users(:kevin).user_devices.find_by!(device_id: "device-a").user_agent
  end

  test "signing in from an unknown device is new_device" do
    UserDevice.record_sign_in!(user: users(:kevin), device_id: "device-a", user_agent: "UA/1")

    assert_equal :new_device, UserDevice.record_sign_in!(user: users(:kevin), device_id: "device-b", user_agent: "UA/1")

    assert_equal %w[ device-a device-b ], users(:kevin).user_devices.order(:device_id).pluck(:device_id)
  end

  test "a blank device id is unknown" do
    assert_equal :unknown, UserDevice.record_sign_in!(user: users(:kevin), device_id: nil, user_agent: "UA/1")
    assert_equal :unknown, UserDevice.record_sign_in!(user: users(:kevin), device_id: "", user_agent: "UA/1")

    assert_empty users(:kevin).user_devices
  end

  test "devices are scoped per user" do
    UserDevice.record_sign_in!(user: users(:kevin), device_id: "device-a", user_agent: "UA/1")

    assert_equal :first_seen, UserDevice.record_sign_in!(user: users(:jz), device_id: "device-a", user_agent: "UA/1")
  end

  test "deactivating wipes devices along with sessions" do
    UserDevice.record_sign_in!(user: users(:kevin), device_id: "device-a", user_agent: "UA/1")

    users(:kevin).deactivate

    assert_empty UserDevice.where(user_id: users(:kevin).id)
  end
end
