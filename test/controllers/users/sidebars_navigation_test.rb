require "test_helper"

class Users::SidebarsNavigationTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "favourites move out of their sections into Favourites" do
    users(:david).memberships.find_by!(room: rooms(:designers)).favorite!
    users(:david).memberships.find_by!(room: rooms(:david_and_jason)).favorite!

    get user_sidebar_url
    assert_response :success

    assert_select "#favorite_rooms" do
      assert_select "a[data-room-id='#{rooms(:designers).id}']", count: 1
      assert_select "a[data-room-id='#{rooms(:david_and_jason).id}']", count: 1
    end
    assert_select "#shared_rooms" do
      assert_select "a[data-room-id='#{rooms(:designers).id}']", count: 0
    end
    assert_select "#direct_rooms" do
      assert_select "a[data-room-id='#{rooms(:david_and_jason).id}']", count: 0
    end
  end

  test "muted rows dim and carry their menu state" do
    users(:david).memberships.find_by!(room: rooms(:designers)).update!(involvement: "muted")

    get user_sidebar_url
    assert_response :success

    assert_select "a##{dom_id(rooms(:designers), :list)}.muted", count: 1
    assert_select "a##{dom_id(rooms(:designers), :list)}[data-menu-muted='true']", count: 1
  end

  test "categorized channels render under their collapsed category" do
    category = users(:david).room_categories.create!(name: "Team", position: 1, collapsed: true)
    users(:david).memberships.find_by!(room: rooms(:designers)).update!(room_category: category)

    get user_sidebar_url
    assert_response :success

    assert_match "Team", @response.body
    assert_select "#category_rooms_#{category.id}" do
      assert_select "a[data-room-id='#{rooms(:designers).id}']", count: 1
    end
    assert_select "#shared_rooms" do
      assert_select "a[data-room-id='#{rooms(:designers).id}']", count: 0
    end
    assert_select "#category_rooms_#{category.id}[hidden]", count: 1
  end

  test "sidebar costs a constant number of queries as favourites and categories grow" do
    seed_sidebar_rooms(offset: 0)

    get user_sidebar_url
    assert_response :success
    # Identical icon-cache state per leg: the custom-icon stamp query
    # re-fires on a one-second monotonic TTL, which a slow gap between
    # the legs would otherwise trip.
    Icons.expire_custom_cache!
    small = count_queries { get user_sidebar_url }

    seed_sidebar_rooms(offset: 10)

    Icons.expire_custom_cache!
    large = count_queries { get user_sidebar_url }
    assert_response :success

    assert_equal small, large,
      "sidebar should be O(1) in queries, got #{small} then #{large}"
  end

  private
    def seed_sidebar_rooms(offset:)
      category = users(:david).room_categories.create!(name: "Seeded #{offset}", position: offset)

      3.times do |i|
        room = Rooms::Closed.create_for({ name: "Seeded room #{offset + i}", creator: users(:david) }, users: [ users(:david) ])
        membership = users(:david).memberships.find_by!(room: room)

        if i.even?
          membership.favorite!
        else
          membership.update!(room_category: category)
        end
      end
    end

    def count_queries
      count = 0
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 unless payload[:name] == "SCHEMA" || payload[:cached]
      end

      ActiveRecord::Base.connection.clear_query_cache
      yield
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end
end
