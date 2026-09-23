require "test_helper"

class Users::ToursControllerTest < ActionDispatch::IntegrationTest
  test "update stamps the tour as completed" do
    sign_in :david
    users(:david).update!(tour_completed_at: nil)

    patch user_tour_url

    assert_response :no_content
    assert_not_nil users(:david).reload.tour_completed_at
  end

  test "update is idempotent" do
    sign_in :david
    users(:david).touch(:tour_completed_at)
    stamped = users(:david).reload.tour_completed_at

    travel_to 1.hour.from_now do
      patch user_tour_url

      assert_response :no_content
      assert_operator users(:david).reload.tour_completed_at, :>, stamped
    end
  end

  test "update requires sign-in" do
    patch user_tour_url

    assert_response :redirect
  end

  test "room pages carry the tour shell for members who never completed it" do
    sign_in :david
    users(:david).update!(tour_completed_at: nil)

    get room_url(rooms(:designers))

    assert_response :success
    assert_select '#tour[data-controller="tour"][data-tour-auto-start-value="true"]'
    assert_select "#help-menu-button", count: 1
  end

  test "room pages skip auto-start once the tour completed" do
    sign_in :david
    users(:david).touch(:tour_completed_at)

    get room_url(rooms(:designers))

    assert_response :success
    assert_select '#tour[data-controller="tour"][data-tour-auto-start-value="false"]'
    assert_select "#help-menu-button", count: 1
  end
end
