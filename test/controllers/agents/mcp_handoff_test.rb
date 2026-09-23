require "test_helper"

class Agents::McpHandoffTest < ActionDispatch::IntegrationTest
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:bender) ])
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
    @receiver_bot = User.create_bot!(name: "Handoff Receiver")
    @receiver = @receiver_bot.create_agent!(kind: :workspace, owner: users(:david))
    @board.memberships.grant_to(@receiver_bot)
    grant!(@agent, "read_messages")
    grant!(@agent, "post_messages")
    grant!(@agent, "manage_threads")
    grant!(@receiver, "read_messages")
    grant!(@receiver, "post_messages")
    grant!(@receiver, "manage_threads")
    @thread = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Ship it", work_status: "in_progress", owner_id: @bot.id)
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
  end

  test "handoff_work hands off through the shared service" do
    body = call_tool("handoff_work", {
      "work_id" => @thread.id, "receiver_agent_id" => @receiver.id,
      "summary" => "Halfway there", "links" => [ "https://example.com/spec" ],
      "open_questions" => [ "Which API?" ]
    })
    content = structured(body)

    assert_equal @receiver_bot.id, content.dig("owner", "id")
    assert_equal "Halfway there", content.dig("handoff", "summary")
    assert_equal [ "https://example.com/spec" ], content.dig("handoff", "links")
    assert_equal @receiver_bot.id, @thread.reload.work_owner_id
    assert_equal "work_handed_off", @receiver.agent_events.deliverable.order(:id).last.event_type
    assert_equal 1, AuditLog.where(action: "work.handoff", target_id: @thread.id).count
  end

  test "handoff_work denies a thread the agent does not own" do
    other = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Mine", work_status: "planned", owner_id: users(:david).id)

    body = call_tool("handoff_work", {
      "work_id" => other.id, "receiver_agent_id" => @receiver.id, "summary" => "Nope"
    })

    assert_tool_error body, "Work not found"
  end

  test "handoff_work denies a missing manage_threads grant" do
    AgentGrant.where(agent: @agent, capability: "manage_threads").update_all(revoked_at: Time.current)

    body = call_tool("handoff_work", {
      "work_id" => @thread.id, "receiver_agent_id" => @receiver.id, "summary" => "Nope"
    })

    assert_tool_error body, "Forbidden: agent lacks manage_threads capability"
  end

  test "handoff_work denies an ineligible receiver" do
    outsider = User.create_bot!(name: "Outsider").create_agent!(kind: :workspace, owner: users(:david))

    body = call_tool("handoff_work", {
      "work_id" => @thread.id, "receiver_agent_id" => outsider.id, "summary" => "Nope"
    })

    assert_tool_error body, "Receiver must be an active agent member of this room with permission to post"
  end

  test "handoff_work denies a receiver missing read_messages" do
    AgentGrant.where(agent: @receiver, capability: "read_messages").update_all(revoked_at: Time.current)

    body = call_tool("handoff_work", {
      "work_id" => @thread.id, "receiver_agent_id" => @receiver.id, "summary" => "Nope"
    })

    assert_tool_error body, "Receiver must hold the read_messages capability in this room"
    assert_equal @bot.id, @thread.reload.work_owner_id
  end

  test "handoff_work denies a kill-switched receiver" do
    @receiver.kill_switch!

    body = call_tool("handoff_work", {
      "work_id" => @thread.id, "receiver_agent_id" => @receiver.id, "summary" => "Nope"
    })

    assert_tool_error body, "Receiver must be an active agent member of this room with permission to post"
    assert_equal @bot.id, @thread.reload.work_owner_id
  end

  test "handoff_work rejects missing arguments" do
    body = call_tool("handoff_work", { "work_id" => @thread.id })

    assert_response :success
    assert_equal(-32602, body.dig("error", "code"))
  end

  test "handoff_work shares its throttle bucket with the rest endpoint" do
    with_memory_cache do
      freeze_time do
        60.times do |index|
          thread = ChannelThread.create_board_post!(room: @board, creator: users(:david),
            name: "Work #{index}", work_status: "planned", owner_id: @bot.id)
          body = call_tool("handoff_work", {
            "work_id" => thread.id, "receiver_agent_id" => @receiver.id, "summary" => "Yours"
          }, id: index + 1)
          assert_equal false, body.dig("result", "isError"), "call #{index} should succeed"
          thread.update_work!(actor: users(:david), work_owner_id: @bot.id)
        end

        overflow = call_tool("handoff_work", {
          "work_id" => @thread.id, "receiver_agent_id" => @receiver.id, "summary" => "One too many"
        }, id: 61)
        assert_equal true, overflow.dig("result", "isError")
        assert_equal "rate_limited", overflow.dig("result", "structuredContent", "error")

        # The REST endpoint draws from the same bucket.
        post agents_work_thread_url(@thread) + "/handoff",
          params: { receiver_agent_id: @receiver.id, summary: "Also throttled" }.to_json,
          headers: bearer_headers
        assert_response :too_many_requests
      end
    end
  end

  private
    def grant!(agent, capability)
      AgentGrant.create!(agent: agent, room: @board, granted_by: users(:david), capability: capability)
    end

    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def mcp_headers(extra = {})
      bearer_headers.merge("MCP-Protocol-Version" => "2025-11-25").merge(extra)
    end

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

    def with_memory_cache
      store = ActiveSupport::Cache::MemoryStore.new
      previous = Rails.cache
      Rails.cache = store
      yield store
    ensure
      Rails.cache = previous
    end
end
