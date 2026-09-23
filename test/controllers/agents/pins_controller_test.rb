require "test_helper"

class Agents::PinsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
    @message = messages(:fourth)
  end

  test "pins with an explicit post_messages grant" do
    grant!(capability: "post_messages", room: @room)

    assert_difference -> { @room.message_pins.count }, 1 do
      post agents_message_pin_url(@message), headers: bearer_headers
    end

    assert_response :created
    assert_equal({ "pinned" => true, "message_id" => @message.id, "pin_count" => 1 }, response.parsed_body)

    pin = @room.message_pins.sole
    assert_equal @bot, pin.pinner
    assert_equal @bot, @room.messages.ordered.last.creator
  end

  test "a legacy agent without any grants can pin" do
    post agents_message_pin_url(@message), headers: bearer_headers

    assert_response :created
    assert_equal @bot, @room.message_pins.sole.pinner
  end

  test "pinning is idempotent" do
    grant!(capability: "post_messages", room: @room)

    post agents_message_pin_url(@message), headers: bearer_headers
    assert_response :created

    assert_no_difference -> { MessagePin.count } do
      post agents_message_pin_url(@message), headers: bearer_headers
    end

    assert_response :success
  end

  test "requires post_messages in the message room" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    post agents_message_pin_url(@message), headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "unpins with post_messages" do
    grant!(capability: "post_messages", room: @room)
    MessagePin.pin!(message: @message, pinner: users(:david))

    assert_difference -> { MessagePin.count }, -1 do
      delete agents_message_pin_url(@message), headers: bearer_headers
    end

    assert_response :success
    assert_equal({ "pinned" => false, "message_id" => @message.id, "pin_count" => 0 }, response.parsed_body)
  end

  test "unpinning requires post_messages too" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")
    MessagePin.pin!(message: @message, pinner: users(:david))

    delete agents_message_pin_url(@message), headers: bearer_headers

    assert_response :forbidden
    assert MessagePin.pinned?(@message)
  end

  test "unpinning a message that is not pinned still succeeds" do
    grant!(capability: "post_messages", room: @room)

    delete agents_message_pin_url(@message), headers: bearer_headers

    assert_response :success
    assert_equal false, response.parsed_body["pinned"]
  end

  test "is 404 for messages the agent cannot see" do
    grant!(capability: "post_messages")

    post agents_message_pin_url(messages(:first)), headers: bearer_headers
    assert_response :not_found

    delete agents_message_pin_url(messages(:first)), headers: bearer_headers
    assert_response :not_found
  end

  test "rejects the room cap" do
    grant!(capability: "post_messages", room: @room)
    MessagePin::MAX_PER_ROOM.times do |n|
      message = @room.root_messages.create!(creator: users(:david), markdown_source: "Pinnable #{n}", client_message_id: "agent-cap-#{n}")
      MessagePin.pin!(message:, pinner: users(:david))
    end

    post agents_message_pin_url(@message), headers: bearer_headers

    assert_response :unprocessable_entity
    assert_equal "This channel already has 50 pinned messages", response.parsed_body["error"]
  end

  test "rejects session requests" do
    sign_in :david

    post agents_message_pin_url(@message)
    assert_response :forbidden
    assert_equal bearer_token_error, response.parsed_body["error"]
  end

  private
    def grant!(capability:, room: nil)
      AgentGrant.create!(agent: @agent, room: room, granted_by: users(:david), capability: capability)
    end

    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def bearer_token_error
      [ "Forbidden: Bearer", "agent", "token required" ].join(" ")
    end
end
