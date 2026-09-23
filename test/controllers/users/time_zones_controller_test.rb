require "test_helper"

class Users::TimeZonesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "detects the browser zone when none is saved" do
    patch user_time_zone_url, params: { time_zone: "Pacific Time (US & Canada)" }, as: :json

    assert_response :success
    assert_equal "Pacific Time (US & Canada)", users(:david).reload.time_zone
    assert_equal "Pacific Time (US & Canada)", response.parsed_body["time_zone"]
  end

  test "a hand-picked zone wins over later detections" do
    users(:david).update!(time_zone: "Eastern Time (US & Canada)")

    patch user_time_zone_url, params: { time_zone: "Pacific Time (US & Canada)" }, as: :json

    assert_response :success
    assert_equal "Eastern Time (US & Canada)", users(:david).reload.time_zone
  end

  test "an unknown zone is ignored" do
    patch user_time_zone_url, params: { time_zone: "Narnia" }, as: :json

    assert_response :success
    assert_nil users(:david).reload.time_zone
  end
end
