require "test_helper"

class Rooms::CategoriesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @category = users(:david).room_categories.create!(name: "Team", position: 1)
    @room = rooms(:designers)
    @membership = users(:david).memberships.find_by!(room: @room)
  end

  test "update assigns the channel to the category" do
    patch room_category_assignment_url(@room, format: :json), params: { room_category_id: @category.id }
    assert_response :success
    assert_equal @category.id, @membership.reload.room_category_id
  end

  test "update with a blank category unassigns the channel" do
    @membership.update!(room_category: @category)

    patch room_category_assignment_url(@room, format: :json), params: { room_category_id: "" }
    assert_response :success
    assert_nil @membership.reload.room_category_id
  end

  test "update rejects another user's category" do
    other = users(:jason).room_categories.create!(name: "Theirs", position: 1)

    assert_raises(ActiveRecord::RecordNotFound) do
      patch room_category_assignment_url(@room, format: :json), params: { room_category_id: other.id }
    end
    assert_nil @membership.reload.room_category_id
  end

  test "update rejects rooms that are not channels" do
    dm = rooms(:david_and_jason)

    patch room_category_assignment_url(dm, format: :json), params: { room_category_id: @category.id }
    assert_response :unprocessable_content
  end

  test "categories in a room the user cannot access are not found" do
    room = Rooms::Closed.create_for({ name: "Secret", creator: users(:jason) }, users: [ users(:jason) ])

    assert_raises(ActiveRecord::RecordNotFound) do
      patch room_category_assignment_url(room, format: :json), params: { room_category_id: @category.id }
    end
  end
end
