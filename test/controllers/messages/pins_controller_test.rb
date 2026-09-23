require "test_helper"

class Messages::PinsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @message = messages(:first)
  end

  test "create pins the message and posts a note" do
    assert_turbo_stream_broadcasts [ @message.room, :messages ], count: 4 do
      assert_difference -> { @message.room.message_pins.count }, 1 do
        assert_difference -> { @message.room.messages.count }, 1 do
          post message_pin_url(@message, format: :json)
        end
      end
    end

    assert_response :created
    assert_equal true, response.parsed_body["pinned"]
    assert_equal 1, response.parsed_body["pin_count"]
  end

  test "create is idempotent for an already-pinned message" do
    MessagePin.pin!(message: @message, pinner: users(:david))

    assert_no_difference -> { MessagePin.count } do
      assert_no_difference -> { @message.room.messages.count } do
        post message_pin_url(@message, format: :json)
      end
    end

    assert_response :success
    assert_equal true, response.parsed_body["pinned"]
  end

  test "create rejects pins past the room cap" do
    MessagePin::MAX_PER_ROOM.times do |n|
      message = @message.room.root_messages.create!(creator: users(:david), markdown_source: "Pinnable #{n}", client_message_id: "controller-cap-#{n}")
      MessagePin.pin!(message:, pinner: users(:david))
    end

    post message_pin_url(@message, format: :json)

    assert_response :unprocessable_entity
    assert_equal "This channel already has 50 pinned messages", response.parsed_body["error"]
  end

  test "destroy unpins the message" do
    MessagePin.pin!(message: @message, pinner: users(:david))
    ActionCable.server.pubsub.clear

    assert_turbo_stream_broadcasts [ @message.room, :messages ], count: 3 do
      assert_difference -> { MessagePin.count }, -1 do
        delete message_pin_url(@message, format: :json)
      end
    end

    assert_response :success
    assert_equal false, response.parsed_body["pinned"]
  end

  test "destroy succeeds when the message is not pinned" do
    assert_no_difference -> { MessagePin.count } do
      delete message_pin_url(@message, format: :json)
    end

    assert_response :success
  end

  test "a non-member cannot pin or unpin" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)
    private_message = private_room.root_messages.create!(creator: users(:jason), markdown_source: "Secret", client_message_id: "private-pin")

    post message_pin_url(private_message, format: :json)
    assert_response :not_found

    MessagePin.pin!(message: private_message, pinner: users(:jason))
    delete message_pin_url(private_message, format: :json)
    assert_response :not_found

    assert MessagePin.pinned?(private_message)
  end
end
