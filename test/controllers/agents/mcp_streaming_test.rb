require "test_helper"

class Agents::McpStreamingTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "start_stream, append_stream, and finalize_stream round-trip" do
    created = call_tool("start_stream", { "room_id" => @room.id, "markdown_source" => "Hello" })
    message_id = structured(created)["id"]
    assert structured(created)["streaming"]
    assert_predicate Message.find(message_id), :streaming?

    appended = call_tool("append_stream", { "message_id" => message_id, "append" => " there" })
    assert_equal "Hello there", structured(appended).dig("body", "plain_text")

    finalized = call_tool("finalize_stream", { "message_id" => message_id })
    assert_equal false, structured(finalized)["streaming"]
    assert_not Message.find(message_id).streaming?
    assert_equal 1, @agent.agent_events.where(event_type: "posted", message_id: message_id).count
  end

  test "start_stream denies without post_messages" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    body = call_tool("start_stream", { "room_id" => @room.id, "markdown_source" => "Denied" })

    assert_tool_error body, "Forbidden: agent lacks post_messages capability"
  end

  test "append_stream denies another author's message" do
    foreign = @room.root_messages.create!(creator: users(:david),
      markdown_source: "Mine", client_message_id: "mcp-stream-foreign")

    body = call_tool("append_stream", { "message_id" => foreign.id, "append" => "Hijack" })

    assert_tool_error body, "Message not found"
  end

  test "finalize_stream refuses a locked thread" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Locked stream")
    ThreadMembership.join!(thread, users(:david))
    created = call_tool("start_stream",
      { "room_id" => @room.id, "thread_id" => thread.id, "markdown_source" => "On it." })
    message_id = structured(created)["id"]
    thread.lock_conversation!

    body = call_tool("finalize_stream", { "message_id" => message_id })

    assert_tool_error body, "This thread is locked"
    assert_predicate Message.find(message_id), :streaming?
  end

  test "message budget denies start_stream with the same 429" do
    @agent.update!(daily_message_cap: 1)
    @room.root_messages.create!(creator: @bot,
      markdown_source: "Spent", client_message_id: "mcp-stream-spent")

    body = call_tool("start_stream", { "room_id" => @room.id, "markdown_source" => "Over" })

    assert_tool_error body, "Daily message budget exceeded (1/day)"
    assert_equal "too_many_requests", body.dig("result", "structuredContent", "status")
  end

  test "set_presence sets and clears working presence" do
    set = call_tool("set_presence", { "text" => "Running tests…" })

    assert_equal "Running tests…", structured(set)["working_presence"]
    assert_equal "Running tests…", @agent.reload.working_presence_text

    cleared = call_tool("set_presence", { "text" => "" })

    assert_nil structured(cleared)["working_presence"]
    assert_nil @agent.reload.working_presence
  end

  test "add_step and update_step round-trip on the agent's message" do
    message = @room.root_messages.create!(creator: @bot,
      markdown_source: "Working", client_message_id: "mcp-step-parent")

    created = call_tool("add_step", { "message_id" => message.id, "name" => "Run tests" })
    step_id = structured(created)["id"]
    assert_equal "running", structured(created)["status"]

    updated = call_tool("update_step", { "step_id" => step_id, "status" => "done",
      "output_summary" => "Green", "duration_ms" => 1500 })
    assert_equal "done", structured(updated)["status"]

    step = AgentStep.find(step_id)
    assert_equal "Green", step.output_summary
    assert_equal 1500, step.duration_ms
  end

  test "add_step denies without post_messages" do
    message = @room.root_messages.create!(creator: @bot,
      markdown_source: "Working", client_message_id: "mcp-step-denied")
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    body = call_tool("add_step", { "message_id" => message.id, "name" => "Denied" })

    assert_tool_error body, "Forbidden: agent lacks post_messages capability"
  end

  test "add_step denies thread steps without manage_threads" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: @bot)
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "post_messages")

    body = call_tool("add_step", { "thread_id" => thread.id, "name" => "Denied" })

    assert_tool_error body, "Forbidden: agent lacks manage_threads capability"
  end

  test "external action budget denies request_approval with 429" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "external_action")
    @agent.update!(daily_external_action_cap: 1)
    AgentApproval.create!(agent: @agent, action: "deploy", summary: "Spent")

    body = call_tool("request_approval", { "action" => "deploy", "summary" => "Over" })

    assert_tool_error body, "Daily external action budget exceeded (1/day)"
  end

  private
    def rpc(method, params = {}, id: 1, headers: {})
      post agents_mcp_url,
        params: { jsonrpc: "2.0", id: id, method: method, params: params }.to_json,
        headers: mcp_headers(headers)

      response.parsed_body
    end

    def call_tool(name, arguments = {}, id: 1, headers: {})
      rpc("tools/call", { "name" => name, "arguments" => arguments }, id: id, headers: headers)
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

    def mcp_headers(extra = {})
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }.merge(extra)
    end
end
