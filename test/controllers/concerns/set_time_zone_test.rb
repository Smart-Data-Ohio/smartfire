require "test_helper"

class TimeZoneProbeController < ApplicationController
  def show
    render plain: Time.zone.name
  end
end

class SetTimeZoneTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    Rails.application.routes.draw do
      get "time_zone_probe", to: "time_zone_probe#show"
    end
  end

  teardown do
    Rails.application.reload_routes!
  end

  test "requests render in the member's time zone" do
    users(:david).update!(time_zone: "Pacific Time (US & Canada)")

    get "/time_zone_probe"

    assert_response :success
    assert_equal "Pacific Time (US & Canada)", response.body
  end

  test "requests without a saved zone keep the default zone" do
    get "/time_zone_probe"

    assert_response :success
    assert_equal Time.zone.name, response.body
  end

  test "an invalid saved zone falls back to the default zone" do
    users(:david).update_column(:time_zone, "Narnia")

    get "/time_zone_probe"

    assert_response :success
    assert_equal Time.zone.name, response.body
  end
end
