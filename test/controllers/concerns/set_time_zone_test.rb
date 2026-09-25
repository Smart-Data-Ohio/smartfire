require "test_helper"

class TimeZoneProbeController < ApplicationController
  def show
    render plain: Time.zone.name
  end
end

class SetTimeZoneTest < ActionDispatch::IntegrationTest
  # Scoped routes for the probe: with_routing swaps in its own route set for
  # each test and restores the application's afterwards, so the global
  # routes are never cleared or reloaded from disk. The set carries the
  # test sign-in route too, since the integration session only sees it.
  with_routing do |set|
    set.draw do
      get "test_session", to: "test_session#create", as: :sign_in_for_tests
      get "time_zone_probe", to: "time_zone_probe#show"
    end
  end

  setup do
    sign_in :david
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

  test "a member zone never leaks past its own request" do
    users(:david).update!(time_zone: "Pacific Time (US & Canada)")

    get "/time_zone_probe"

    assert_response :success
    assert_equal "Pacific Time (US & Canada)", response.body
    assert_equal Time.zone_default.name, Time.zone.name
  end
end
