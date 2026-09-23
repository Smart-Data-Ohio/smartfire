require "test_helper"

class Agents::McpSlashPollsTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "register_slash_command registers through the shared service" do
    grant!(capability: "post_messages", room: @room)

    body = call_tool("register_slash_command",
      { "room_id" => @room.id, "name" => "deploy", "description" => "Ship it" })

    payload = structured(body)
    assert_equal "deploy", payload["name"]
    assert AgentSlashCommand.exists?(agent: @agent, room: @room, name: "deploy")
  end

  test "register_slash_command denies without post_messages" do
    grant!(capability: "read_messages", room: @room)

    body = call_tool("register_slash_command", { "room_id" => @room.id, "name" => "deploy" })

    assert_tool_error body, "Forbidden: agent lacks post_messages capability"
  end

  test "register_slash_command rejects built-in names like REST" do
    grant!(capability: "post_messages", room: @room)

    body = call_tool("register_slash_command", { "room_id" => @room.id, "name" => "poll" })

    assert_tool_error body, "Name is already a built-in command"
  end

  test "unregister_slash_command removes the agent's own command" do
    grant!(capability: "post_messages", room: @room)
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")

    body = call_tool("unregister_slash_command", { "room_id" => @room.id, "name" => "deploy" })

    assert_equal true, structured(body)["unregistered"]
    assert_not AgentSlashCommand.exists?(room: @room, name: "deploy")
  end

  test "unregister_slash_command denies without post_messages" do
    grant!(capability: "read_messages", room: @room)

    body = call_tool("unregister_slash_command", { "room_id" => @room.id, "name" => "deploy" })

    assert_tool_error body, "Forbidden: agent lacks post_messages capability"
  end

  test "create_poll posts through the shared service" do
    grant!(capability: "post_messages", room: @room)

    body = call_tool("create_poll",
      { "room_id" => @room.id, "question" => "Lunch?", "options" => [ "Tacos", "Pizza" ] })

    payload = structured(body)
    assert_equal "Lunch?", payload["question"]
    assert_equal 2, payload["options"].size
    assert_equal users(:bender).id, Poll.order(:id).last.message.creator_id
  end

  test "create_poll denies without post_messages" do
    grant!(capability: "read_messages", room: @room)

    body = call_tool("create_poll",
      { "room_id" => @room.id, "question" => "Lunch?", "options" => [ "Tacos", "Pizza" ] })

    assert_tool_error body, "Forbidden: agent lacks post_messages capability"
  end

  test "get_poll reads live results" do
    grant!(capability: "post_messages", room: @room)
    poll = create_poll
    poll.cast_vote!(users(:david), [ poll.poll_options.first.id ])

    body = call_tool("get_poll", { "room_id" => @room.id, "poll_id" => poll.id })

    payload = structured(body)
    assert_equal 1, payload["total_votes"]
    assert_equal [ "David" ], payload["options"].first["voters"]
  end

  test "get_poll denies without post_messages" do
    grant!(capability: "read_messages", room: @room)
    poll = create_poll

    body = call_tool("get_poll", { "room_id" => @room.id, "poll_id" => poll.id })

    assert_tool_error body, "Forbidden: agent lacks post_messages capability"
  end

  test "invoked commands poll as slash_command events" do
    grant!(capability: "post_messages", room: @room)
    grant!(capability: "read_messages", room: @room)
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")

    SlashCommands::Dispatcher.dispatch(user: users(:david), room: @room, text: "/deploy staging")

    body = call_tool("poll_events", {})
    events = structured(body)["events"]
    command = events.find { |event| event["event_type"] == "slash_command" }

    assert_not_nil command, "expected a slash_command event in #{events.map { |event| event["event_type"] }.inspect}"
    assert_equal "deploy", command.dig("command", "name")
    assert_equal "staging", command.dig("command", "arguments")
    assert_equal "David", command.dig("actor", "name")
    assert_equal @room.id, command.dig("room", "id")
  end

  test "slash_command events ack under the standard read gate" do
    grant!(capability: "post_messages", room: @room)
    grant!(capability: "read_messages", room: @room)
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")

    SlashCommands::Dispatcher.dispatch(user: users(:david), room: @room, text: "/deploy staging")
    event = @agent.agent_events.ordered.last

    body = call_tool("ack_events", { "event_ids" => [ event.id ] })

    assert_equal "acknowledged", structured(body)["results"].first["outcome"]
  end

  private
    def create_poll(**options)
      message = @room.root_messages.create!(creator: users(:david), markdown_source: "Lunch?")
      Poll.create_for_message!(message: message, labels: [ "Tacos", "Pizza" ], **options)
    end

    def call_tool(name, arguments = {}, id: 1)
      post agents_mcp_url,
        params: { jsonrpc: "2.0", id: id, method: "tools/call", params: { "name" => name, "arguments" => arguments } }.to_json,
        headers: { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }

      response.parsed_body
    end

    def structured(body)
      assert_response :success
      assert_equal false, body.dig("result", "isError"), "expected tool success, got: #{body.dig("result", "content")&.first&.dig("text")}"

      body.dig("result", "structuredContent")
    end

    def assert_tool_error(body, message)
      assert_response :success
      assert_equal true, body.dig("result", "isError"), "expected tool error, got success: #{body}"
      assert_equal message, body.dig("result", "content").first["text"]
      assert_equal message, body.dig("result", "structuredContent", "error")
    end

    def grant!(capability:, room: nil)
      AgentGrant.create!(agent: @agent, room: room, granted_by: users(:david), capability: capability)
    end
end
