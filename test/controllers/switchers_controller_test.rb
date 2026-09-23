require "test_helper"

class SwitchersControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "show returns the user's rooms, people and recent threads" do
    room = rooms(:designers)
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "Launch plan")

    get switcher_url(format: :json)
    assert_response :success

    payload = response.parsed_body
    room_names = payload.fetch("rooms").map { |entry| entry["name"] }
    assert_includes room_names, "Designers"
    assert_includes room_names, "HQ"

    entry = payload["rooms"].find { |candidate| candidate["name"] == "Designers" }
    assert_equal room.id, entry["id"]
    assert_equal "channel", entry["kind"]
    assert_equal room_path(room), entry["url"]

    people_names = payload.fetch("people").map { |entry| entry["name"] }
    assert_includes people_names, "Jason"
    assert_not_includes people_names, "David"

    thread_names = payload.fetch("threads").map { |entry| entry["name"] }
    assert_includes thread_names, "Launch plan"
  end

  test "show never includes rooms the user cannot access" do
    outsider_closed = Rooms::Closed.create_for({ name: "Secret", creator: users(:jason) }, users: [ users(:jason) ])
    outsider_dm = Rooms::Direct.create_for({ creator: users(:jason) }, users: [ users(:jason), users(:kevin) ])
    hidden = rooms(:designers)
    users(:david).memberships.find_by!(room: hidden).update!(involvement: "invisible")

    get switcher_url(format: :json)
    assert_response :success

    room_ids = response.parsed_body.fetch("rooms").map { |entry| entry["id"] }
    assert_not_includes room_ids, outsider_closed.id
    assert_not_includes room_ids, outsider_dm.id
    assert_not_includes room_ids, hidden.id
  end

  test "show links people to their existing DM" do
    dm = rooms(:david_and_jason)

    get switcher_url(format: :json)
    assert_response :success

    jason = response.parsed_body.fetch("people").find { |entry| entry["name"] == "Jason" }
    assert_equal room_path(dm), jason["dm_url"]

    jz = response.parsed_body.fetch("people").find { |entry| entry["name"] == "JZ" }
    assert_nil jz["dm_url"]
  end

  test "show requires sign-in" do
    sign_out

    get switcher_url(format: :json)
    assert_response :redirect
  end

  test "show costs a constant number of queries as rooms, people and threads grow" do
    seed_switcher_data(offset: 0)

    get switcher_url(format: :json)
    assert_response :success
    small = count_queries { get switcher_url(format: :json) }

    seed_switcher_data(offset: 10)

    large = count_queries { get switcher_url(format: :json) }
    assert_response :success

    assert_equal small, large,
      "switcher should be O(1) in queries, got #{small} then #{large}"
  end

  private
    def seed_switcher_data(offset:)
      3.times do |i|
        room = Rooms::Closed.create_for({ name: "Extra #{offset + i}", creator: users(:david) }, users: [ users(:david), users(:jason) ])
        ChannelThread.create!(room: room, creator: users(:david), name: "Thread #{offset + i}")
      end
    end

    def sign_out
      delete session_url
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
