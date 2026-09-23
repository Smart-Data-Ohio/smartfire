require "test_helper"

class RoomCategoriesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "create adds a category at the end" do
    users(:david).room_categories.create!(name: "First", position: 1)

    assert_difference -> { users(:david).room_categories.count } do
      post room_categories_url, params: { room_category: { name: "Team" } }
      assert_redirected_to user_sidebar_url
    end

    category = users(:david).room_categories.order(:id).last
    assert_equal "Team", category.name
    assert_equal 2, category.position
    assert_not category.collapsed?
  end

  test "create with a blank name changes nothing and reloads the sidebar" do
    assert_no_difference -> { RoomCategory.count } do
      post room_categories_url, params: { room_category: { name: "" } }
      assert_redirected_to user_sidebar_url
    end
  end

  test "update renames and collapses" do
    category = users(:david).room_categories.create!(name: "Team", position: 1)

    patch room_category_url(category), params: { room_category: { name: "Squad", collapsed: true } }
    assert_redirected_to user_sidebar_url

    category.reload
    assert_equal "Squad", category.name
    assert category.collapsed?
  end

  test "destroy deletes the category and unassigns its rooms" do
    category = users(:david).room_categories.create!(name: "Team", position: 1)
    membership = users(:david).memberships.find_by!(room: rooms(:designers))
    membership.update!(room_category: category)

    delete room_category_url(category)
    assert_redirected_to user_sidebar_url

    assert_not RoomCategory.exists?(category.id)
    assert_nil membership.reload.room_category_id
  end

  test "another user's categories are not found" do
    category = users(:jason).room_categories.create!(name: "Theirs", position: 1)

    assert_raises(ActiveRecord::RecordNotFound) do
      patch room_category_url(category), params: { room_category: { name: "Mine" } }
    end

    assert_raises(ActiveRecord::RecordNotFound) do
      delete room_category_url(category)
    end

    assert_equal "Theirs", category.reload.name
  end
end
