require "test_helper"

class MessageForwardsControllerTest < ActionDispatch::IntegrationTest
  setup do
    host! "smartfire.test"
    sign_in :jz
    @room = rooms(:designers)
    @message = messages(:third)
  end

  test "destinations returns reachable rooms and unlocked threads without caching" do
    open_thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "Open destination")
    locked_thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "Locked destination")
    locked_thread.lock_conversation!

    get room_message_forward_destinations_url(@room, @message, format: :json)

    assert_response :success
    assert_equal "no-store", response.headers["Cache-Control"]

    destinations = response.parsed_body.fetch("destinations")
    designers = destinations.find { |destination| destination.fetch("room_id") == @room.id }
    assert designers
    assert_equal false, designers.fetch("direct")
    assert_includes designers.fetch("threads").map { |thread| thread.fetch("id") }, open_thread.id
    assert_not_includes designers.fetch("threads").map { |thread| thread.fetch("id") }, locked_thread.id
    assert destinations.none? { |destination| destination.fetch("room_id") == rooms(:watercooler).id }, "unreachable rooms must not be offered"
  end

  test "nested destination endpoint supports a thread message" do
    thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "Nested source")
    source = thread.post_message!(creator: users(:jz), attributes: { markdown_source: "Nested source", client_message_id: "nested-source" })

    get room_thread_message_forward_destinations_url(@room, thread, source, format: :json)

    assert_response :success
    assert response.parsed_body.key?("destinations")
  end

  test "destinations excludes board rooms" do
    board = Rooms::Board.create_for({ name: "Launch", creator: users(:jz) }, users: [ users(:jz) ])

    get room_message_forward_destinations_url(@room, @message, format: :json)

    assert_response :success
    destinations = response.parsed_body.fetch("destinations")
    assert destinations.none? { |destination| destination.fetch("room_id") == board.id }, "boards must not be offered"
    assert destinations.any? { |destination| destination.fetch("room_id") == @room.id }, "channels are still offered"
  end

  test "create refuses board destinations on the server" do
    board = Rooms::Board.create_for({ name: "Launch", creator: users(:jz) }, users: [ users(:jz) ])
    board_post = ChannelThread.create!(room: board, creator: users(:jz), name: "Ship it", work_status: "planned")

    # A thread destination: root forwards into boards already fail on the
    # message validation, so only a post destination exercises the new check.
    assert_no_difference -> { Message.count } do
      post room_message_forwards_url(@room, @message, format: :json), params: {
        forward: { destinations: [ { room_id: board.id, thread_id: board_post.id } ] }
      }
    end

    assert_response :unprocessable_content
    assert_match(/board/i, response.parsed_body.fetch("error"))
  end

  test "direct destinations use the other participant's display name" do
    direct = Current.set(user: users(:jz)) do
      Rooms::Direct.find_or_create_for([ users(:jz), users(:kevin) ])
    end
    direct.update_column(:name, "Internal direct room name")

    get room_message_forward_destinations_url(@room, @message, format: :json)

    assert_response :success
    destination = response.parsed_body.fetch("destinations").find { |candidate| candidate.fetch("room_id") == direct.id }
    assert_equal "Kevin", destination.fetch("name")
  end
end
