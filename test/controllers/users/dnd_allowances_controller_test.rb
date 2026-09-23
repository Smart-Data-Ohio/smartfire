require "test_helper"

class Users::DndAllowancesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "starring and unstarring someone for DND" do
    post user_dnd_allowance_url(users(:jason))

    assert_redirected_to user_url(users(:jason))
    assert users(:david).reload.dnd_allows?(users(:jason))

    delete user_dnd_allowance_url(users(:jason))

    assert_redirected_to user_url(users(:jason))
    assert_not users(:david).reload.dnd_allows?(users(:jason))
  end

  test "starring twice stays a single exception" do
    2.times { post user_dnd_allowance_url(users(:jason)) }

    assert_equal 1, users(:david).dnd_allowed_users.count
  end

  test "cannot star yourself" do
    post user_dnd_allowance_url(users(:david))

    assert_response :unprocessable_entity
    assert_empty users(:david).dnd_allowed_users
  end

  test "cannot star a bot" do
    post user_dnd_allowance_url(users(:bender))

    assert_response :not_found
    assert_empty users(:david).dnd_allowed_users
  end
end
