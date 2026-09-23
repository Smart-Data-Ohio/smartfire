require "test_helper"

class Rooms::FavoritesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
    @membership = users(:david).memberships.find_by!(room: @room)
  end

  test "create favourites the room at the end" do
    rooms(:hq).memberships.find_by(user: users(:david))&.favorite!

    post room_favorite_url(@room, format: :json)
    assert_response :success

    assert_predicate @membership.reload, :favorited?
    assert_equal 1, @membership.favorite_position
  end

  test "create is idempotent" do
    @membership.favorite!

    assert_no_difference -> { users(:david).memberships.favorites.count } do
      post room_favorite_url(@room, format: :json)
      assert_response :success
    end
  end

  test "destroy unfavourites the room" do
    @membership.favorite!

    delete room_favorite_url(@room, format: :json)
    assert_response :success

    assert_not @membership.reload.favorited?
  end

  test "update moves the favourite to an absolute position" do
    hq = users(:david).memberships.find_by!(room: rooms(:hq))
    pets = users(:david).memberships.find_by!(room: rooms(:pets))
    [ @membership, hq, pets ].each(&:favorite!)

    patch room_favorite_url(rooms(:pets), format: :json), params: { position: 0 }
    assert_response :success

    assert_equal [ pets.id, @membership.id, hq.id ], users(:david).memberships.favorites.pluck(:id)
  end

  test "update clamps out-of-range positions" do
    @membership.favorite!

    patch room_favorite_url(@room, format: :json), params: { position: 99 }
    assert_response :success
    assert_equal 0, @membership.reload.favorite_position
  end

  test "favourites in a room the user cannot access are not found" do
    room = Rooms::Closed.create_for({ name: "Secret", creator: users(:jason) }, users: [ users(:jason) ])

    assert_raises(ActiveRecord::RecordNotFound) do
      post room_favorite_url(room, format: :json)
    end

    assert_raises(ActiveRecord::RecordNotFound) do
      delete room_favorite_url(room, format: :json)
    end

    assert_raises(ActiveRecord::RecordNotFound) do
      patch room_favorite_url(room, format: :json), params: { position: 0 }
    end
  end
end
