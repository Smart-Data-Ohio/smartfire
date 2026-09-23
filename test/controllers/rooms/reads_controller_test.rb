require "test_helper"

class Rooms::ReadsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
    @membership = users(:david).memberships.find_by!(room: @room)
  end

  test "create marks the room read and advances the unread pointer" do
    first, second = @room.root_messages.ordered.first(2)
    @membership.update!(unread_at: second.created_at, last_read_message_id: first.id)

    post room_read_url(@room, format: :json)
    assert_response :success

    assert_not @membership.reload.unread?
    assert_equal @room.root_messages.ordered.last.id, @membership.last_read_message_id
    assert_equal false, response.parsed_body["unread"]
  end

  test "create broadcasts on the reads stream" do
    assert_broadcasts "user_#{users(:david).id}_reads", 1 do
      post room_read_url(@room, format: :json)
      assert_response :success
    end
  end

  test "destroy marks the room unread starting at the message" do
    @membership.read
    target = @room.root_messages.ordered.second

    delete room_read_url(@room, format: :json), params: { message_id: target.id }
    assert_response :success

    @membership.reload
    assert_predicate @membership, :unread?
    assert_equal target.id, @membership.first_unread_message.id
    assert_equal target.id, response.parsed_body["first_unread_message_id"]
  end

  test "destroy at the first message leaves a null pointer" do
    @membership.read
    target = @room.root_messages.ordered.first

    delete room_read_url(@room, format: :json), params: { message_id: target.id }
    assert_response :success

    assert_nil @membership.reload.last_read_message_id
    assert_equal target.id, @membership.first_unread_message.id
  end

  test "destroy broadcasts on the unread stream" do
    target = @room.root_messages.ordered.first

    assert_broadcasts UnreadRoomsChannel.stream_name_for(users(:david).id), 1 do
      delete room_read_url(@room, format: :json), params: { message_id: target.id }
      assert_response :success
    end
  end

  test "destroy rejects messages outside the room" do
    other_message = rooms(:watercooler).root_messages.ordered.first

    assert_raises(ActiveRecord::RecordNotFound) do
      delete room_read_url(@room, format: :json), params: { message_id: other_message.id }
    end
  end

  test "read state of a room the user cannot access is not found" do
    room = Rooms::Closed.create_for({ name: "Secret", creator: users(:jason) }, users: [ users(:jason) ])

    assert_raises(ActiveRecord::RecordNotFound) do
      post room_read_url(room, format: :json)
    end

    assert_raises(ActiveRecord::RecordNotFound) do
      delete room_read_url(room, format: :json), params: { message_id: 1 }
    end
  end

  private
    def assert_broadcasts(stream, count)
      before = ActionCable.server.pubsub.broadcasts(stream).size
      yield
      assert_equal count, ActionCable.server.pubsub.broadcasts(stream).size - before
    end
end
