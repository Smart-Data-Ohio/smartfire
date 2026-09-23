require "test_helper"

class Agents::McpFizzyTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "list_fizzy_boards returns the owner's boards" do
    account = link_owner_fizzy!
    grant!(capability: "fizzy")
    stub = stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .with(headers: { "Authorization" => "Bearer #{account.access_token}" })
      .to_return(status: 200, body: [ fizzy_board_payload ].to_json)

    content = structured(call_tool("list_fizzy_boards", {}))

    assert_requested stub
    assert_equal "Engineering", content.first["name"]
  end

  test "list_fizzy_boards accepts an account_id override" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub = stub_request(:get, "https://app.fizzy.do/123456789/boards.json")
      .to_return(status: 200, body: [ fizzy_board_payload(name: "Other") ].to_json)

    content = structured(call_tool("list_fizzy_boards", { "account_id" => "123456789" }))

    assert_requested stub
    assert_equal "Other", content.first["name"]
  end

  test "get_fizzy_board returns the board with its columns" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub_request(:get, "https://app.fizzy.do/897362094/boards/03board1.json")
      .to_return(status: 200, body: fizzy_board_payload.to_json)
    stub_request(:get, "https://app.fizzy.do/897362094/boards/03board1/columns.json")
      .to_return(status: 200, body: [ fizzy_column_payload ].to_json)

    content = structured(call_tool("get_fizzy_board", { "board_id" => "03board1" }))

    assert_equal "Engineering", content["board"]["name"]
    assert_equal "In Progress", content["columns"].first["name"]
  end

  test "search_fizzy_cards returns matching cards" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub = stub_request(:get, "https://app.fizzy.do/897362094/search.json?q=billing")
      .to_return(status: 200, body: [ fizzy_card_payload ].to_json)

    content = structured(call_tool("search_fizzy_cards", { "q" => "billing" }))

    assert_requested stub
    assert_equal "Fix the billing bug", content.first["title"]
  end

  test "get_fizzy_card returns one card with steps" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub_fizzy_card(579)

    content = structured(call_tool("get_fizzy_card", { "account_id" => "897362094", "number" => 579 }))

    assert_equal "Fix the billing bug", content["title"]
    assert_equal "Reproduce", content["steps"].first["content"]
  end

  test "reads without the fizzy capability are denied before touching Fizzy" do
    link_owner_fizzy!

    assert_tool_error call_tool("list_fizzy_boards", {}), "Forbidden: agent lacks fizzy capability"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a room-scoped fizzy grant denies reads: they need it workspace-wide" do
    link_owner_fizzy!
    grant!(capability: "fizzy", room: rooms(:watercooler))

    assert_tool_error call_tool("get_fizzy_card", { "account_id" => "897362094", "number" => 579 }),
      "Forbidden: agent lacks fizzy capability"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "reads without a linked owner account fail without touching Fizzy" do
    grant!(capability: "fizzy")

    assert_tool_error call_tool("list_fizzy_boards", {}), "Agent owner has no usable Fizzy account"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "an unknown board reads as not found" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub_request(:get, "https://app.fizzy.do/897362094/boards/03missing.json")
      .to_return(status: 404, body: {}.to_json)

    assert_tool_error call_tool("get_fizzy_board", { "board_id" => "03missing" }), "Not found in Fizzy"
  end

  test "a forbidden card reads as not found" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json")
      .to_return(status: 403, body: {}.to_json)

    assert_tool_error call_tool("get_fizzy_card", { "account_id" => "897362094", "number" => 579 }), "Not found in Fizzy"
  end

  test "an invalid board id fails without a request" do
    link_owner_fizzy!
    grant!(capability: "fizzy")

    assert_tool_error call_tool("get_fizzy_board", { "board_id" => "03board!x" }), "Not found in Fizzy"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "an invalid card number fails without a request" do
    link_owner_fizzy!
    grant!(capability: "fizzy")

    assert_tool_error call_tool("get_fizzy_card", { "account_id" => "897362094", "number" => "abc" }), "Invalid card number"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a rejected owner token disconnects the account" do
    account = link_owner_fizzy!
    grant!(capability: "fizzy")
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .to_return(status: 401, body: {}.to_json)

    assert_tool_error call_tool("list_fizzy_boards", {}), "Agent owner's Fizzy token was rejected"
    assert_not account.reload.connected?
  end

  test "a Fizzy outage fails the read" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json").to_return(status: 500, body: "boom")

    body = call_tool("list_fizzy_boards", {})

    assert_response :success
    assert_equal true, body.dig("result", "isError")
    assert_equal "bad_gateway", body.dig("result", "structuredContent", "status")
  end

  test "reads with missing arguments are invalid params" do
    link_owner_fizzy!
    grant!(capability: "fizzy")

    body = call_tool("get_fizzy_board", {})
    assert_equal(-32602, body.dig("error", "code"))

    body = call_tool("search_fizzy_cards", {})
    assert_equal(-32602, body.dig("error", "code"))

    body = call_tool("get_fizzy_card", { "account_id" => "897362094" })
    assert_equal(-32602, body.dig("error", "code"))

    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "create_fizzy_card requests approval without calling Fizzy" do
    account = link_owner_fizzy!
    grant!(capability: "external_action")

    content = nil
    assert_difference -> { AgentApproval.count }, 1 do
      content = structured(call_tool("create_fizzy_card",
        { "board_id" => "03board1", "title" => "Ship it", "description" => "Everything" }))
    end

    assert_not_requested :any, %r{app\.fizzy\.do}

    approval = AgentApproval.last
    assert_equal "fizzy.create", approval.action
    assert_equal approval.id, content["id"]
    assert_equal "pending", content["status"]
    assert_equal account.id, approval.fizzy_connected_account_id
    assert_nil approval.room_id
  end

  test "comment_on_fizzy_card requests approval without calling Fizzy" do
    link_owner_fizzy!
    grant!(capability: "external_action")

    content = nil
    assert_difference -> { AgentApproval.count }, 1 do
      content = structured(call_tool("comment_on_fizzy_card",
        { "account_id" => "897362094", "number" => 579, "body" => "Nice work" }))
    end

    assert_not_requested :any, %r{app\.fizzy\.do}
    assert_equal "fizzy.comment", AgentApproval.last.action
    assert_equal AgentApproval.last.id, content["id"]
  end

  test "move_fizzy_card requests approval without calling Fizzy" do
    link_owner_fizzy!
    grant!(capability: "external_action")

    assert_difference -> { AgentApproval.count }, 1 do
      structured(call_tool("move_fizzy_card",
        { "account_id" => "897362094", "number" => 579, "column_id" => "03column1" }))
    end

    assert_not_requested :any, %r{app\.fizzy\.do}
    assert_equal "fizzy.move", AgentApproval.last.action
  end

  test "close_fizzy_card and reopen_fizzy_card request approval without calling Fizzy" do
    link_owner_fizzy!
    grant!(capability: "external_action")

    assert_difference -> { AgentApproval.count }, 1 do
      structured(call_tool("close_fizzy_card", { "account_id" => "897362094", "number" => 579 }))
    end
    assert_equal "fizzy.close", AgentApproval.last.action

    assert_difference -> { AgentApproval.count }, 1 do
      structured(call_tool("reopen_fizzy_card", { "account_id" => "897362094", "number" => 579 }))
    end
    assert_equal "fizzy.reopen", AgentApproval.last.action

    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "writes without workspace-wide external_action are denied" do
    link_owner_fizzy!
    grant!(capability: "external_action", room: rooms(:watercooler))

    assert_no_difference -> { AgentApproval.count } do
      assert_tool_error call_tool("close_fizzy_card", { "account_id" => "897362094", "number" => 579 }),
        "Forbidden: agent lacks external_action capability"
    end
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "writes without a linked owner account fail" do
    grant!(capability: "external_action")

    assert_no_difference -> { AgentApproval.count } do
      assert_tool_error call_tool("close_fizzy_card", { "account_id" => "897362094", "number" => 579 }),
        "Agent owner has no usable Fizzy account"
    end
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a repeated external_id returns the existing request" do
    link_owner_fizzy!
    grant!(capability: "external_action")
    arguments = { "account_id" => "897362094", "number" => 579, "external_id" => "mcp-fizzy-1" }

    first = structured(call_tool("close_fizzy_card", arguments))

    assert_no_difference -> { AgentApproval.count } do
      second = structured(call_tool("close_fizzy_card", arguments))

      assert_equal first["id"], second["id"]
    end
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "invalid write input fails without creating an approval" do
    link_owner_fizzy!
    grant!(capability: "external_action")

    assert_no_difference -> { AgentApproval.count } do
      assert_tool_error call_tool("move_fizzy_card",
        { "account_id" => "897362094", "number" => 579, "column_id" => "nope!bad" }),
        "Column is invalid"
    end
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "writes with missing arguments are invalid params" do
    link_owner_fizzy!
    grant!(capability: "external_action")

    assert_no_difference -> { AgentApproval.count } do
      body = call_tool("create_fizzy_card", { "board_id" => "03board1" })
      assert_equal(-32602, body.dig("error", "code"))

      body = call_tool("comment_on_fizzy_card", { "number" => 579 })
      assert_equal(-32602, body.dig("error", "code"))

      body = call_tool("close_fizzy_card", {})
      assert_equal(-32602, body.dig("error", "code"))
    end
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "fizzy.* approvals cannot be requested through request_approval" do
    link_owner_fizzy!
    grant!(capability: "external_action")

    assert_no_difference -> { AgentApproval.count } do
      assert_tool_error call_tool("request_approval", { "action" => "fizzy.comment", "summary" => "Sneaky" }),
        "fizzy.* actions are requested through /agents/fizzy/card_actions"
    end
  end

  test "an approved MCP comment completes and polls through poll_events" do
    link_owner_fizzy!
    grant!(capability: "external_action")
    grant!(capability: "read_messages")
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)

    content = structured(call_tool("comment_on_fizzy_card",
      { "account_id" => "897362094", "number" => 579, "body" => "Nice work" }))
    approval = AgentApproval.find(content["id"])

    perform_enqueued_jobs only: Fizzy::PerformAgentActionJob do
      approval.decide!(decision: "approved", by: users(:david))
    end

    events = structured(call_tool("poll_events", {}))["events"]
    completion = events.find { |event| event["event_type"] == "fizzy_action_completed" }

    assert_equal "completed", completion["fizzy_action"]["status"]
    assert_equal "fizzy.comment", completion["fizzy_action"]["action"]
    assert_equal "https://app.fizzy.do/897362094/cards/579/comments/03comment1", completion["fizzy_action"]["url"]
    assert_equal approval.id, completion["fizzy_action"]["approval_id"]
  end

  test "Fizzy reads share buckets with their REST endpoints" do
    link_owner_fizzy!
    grant!(capability: "fizzy")
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .to_return(status: 200, body: [ fizzy_board_payload ].to_json)

    with_memory_cache do
      freeze_time do
        120.times do
          get "/agents/fizzy/boards", headers: mcp_headers
          assert_response :success
        end

        body = call_tool("list_fizzy_boards", {})

        assert_equal true, body.dig("result", "isError")
        assert_equal "rate_limited", body.dig("result", "structuredContent", "error")
      end
    end
  end

  test "Fizzy writes share buckets with their REST endpoint" do
    link_owner_fizzy!
    grant!(capability: "external_action")

    with_memory_cache do
      freeze_time do
        60.times do |index|
          post "/agents/fizzy/card_actions",
            params: { kind: "close", account_id: "897362094", number: 579 + index }.to_json,
            headers: mcp_headers
          assert_response :accepted
        end

        body = call_tool("close_fizzy_card", { "account_id" => "897362094", "number" => 579 })

        assert_equal true, body.dig("result", "isError")
        assert_equal "rate_limited", body.dig("result", "structuredContent", "error")
      end
    end
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

    def grant!(capability:, room: nil)
      AgentGrant.create!(agent: @agent, room: room, granted_by: users(:david), capability: capability)
    end

    def link_owner_fizzy!(token: "owner-token-abc")
      link_fizzy!(users(:david), token: token)
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
