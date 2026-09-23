require "test_helper"

class Agents::McpControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "end-to-end initialize, tools/list, tools/call sequence" do
    init = rpc("initialize", { "protocolVersion" => "2025-11-25", "capabilities" => {}, "clientInfo" => { "name" => "test", "version" => "1" } }, id: 1)
    assert_response :success
    assert_equal "2025-11-25", init.dig("result", "protocolVersion")

    list = rpc("tools/list", {}, id: 2)
    assert_response :success
    names = list.dig("result", "tools").map { |tool| tool["name"] }
    assert_includes names, "post_message"

    created = call_tool("post_message", { "room_id" => @room.id, "markdown_source" => "Hello from MCP" }, id: 3)
    assert_response :success
    assert_equal false, created.dig("result", "isError")
    message = @room.messages.order(:id).last
    assert_equal "Hello from MCP", message.plain_text_body
    assert_equal @bot.id, message.creator_id
    assert_equal message.id, created.dig("result", "structuredContent", "id")
  end

  test "tools/list returns every tool with a schema in stable order" do
    first = rpc("tools/list", {}, id: 1)
    second = rpc("tools/list", {}, id: 2)

    assert_equal first["result"], second["result"]

    names = first.dig("result", "tools").map { |tool| tool["name"] }
    assert_equal %w[
      list_rooms read_messages post_message react poll_events ack_events
      list_board_posts create_board_post update_board_post set_result
      list_work update_work request_approval get_approval get_context open_dm
      pin_message unpin_message
      register_slash_command unregister_slash_command create_poll get_poll
    ], names

    first.dig("result", "tools").each do |tool|
      assert tool["description"].present?, "#{tool["name"]} needs a description"
      assert_equal "object", tool.dig("inputSchema", "type"), "#{tool["name"]} needs an object schema"
    end
  end

  test "initialize echoes a supported version with capabilities and server info" do
    body = rpc("initialize", { "protocolVersion" => "2025-06-18", "capabilities" => {}, "clientInfo" => { "name" => "c", "version" => "1" } })

    assert_response :success
    assert_equal "2025-06-18", body.dig("result", "protocolVersion")
    assert_equal({ "listChanged" => false }, body.dig("result", "capabilities", "tools"))
    assert_equal "smartfire", body.dig("result", "serverInfo", "name")
    assert body.dig("result", "serverInfo", "version").present?
    assert body.dig("result", "instructions").present?
  end

  test "initialize rejects unknown versions with the supported list" do
    body = rpc("initialize", { "protocolVersion" => "1999-01-01", "capabilities" => {}, "clientInfo" => { "name" => "c", "version" => "1" } })

    assert_response :bad_request
    assert_equal(-32022, body.dig("error", "code"))
    assert_equal Agents::McpServer::SUPPORTED_VERSIONS, body.dig("error", "data", "supported")
    assert_equal "1999-01-01", body.dig("error", "data", "requested")
  end

  test "unversioned requests read as legacy and skip header validation" do
    body = rpc("tools/list", {})

    assert_response :success
    assert_not_nil body["result"]["tools"]
    assert_nil body["result"]["resultType"]
  end

  test "server/discover answers modern clients with the full envelope" do
    body = rpc("server/discover", modern_meta, headers: modern_headers("server/discover"))

    assert_response :success
    assert_equal "complete", body.dig("result", "resultType")
    assert_equal Agents::McpServer::SUPPORTED_VERSIONS, body.dig("result", "supportedVersions")
    assert_equal({}, body.dig("result", "capabilities", "tools"))
    assert_equal "smartfire", body.dig("result", "_meta", "io.modelcontextprotocol/serverInfo", "name")
    assert body.dig("result", "instructions").present?
  end

  test "modern tools/list carries resultType" do
    body = rpc("tools/list", modern_meta, headers: modern_headers("tools/list"))

    assert_response :success
    assert_equal "complete", body.dig("result", "resultType")
    assert_equal 18, body.dig("result", "tools").size
  end

  test "header and body versions must match" do
    body = rpc("tools/list", modern_meta("2025-11-25"), headers: modern_headers("tools/list"))

    assert_response :bad_request
    assert_equal(-32020, body.dig("error", "code"))
  end

  test "modern requests require matching Mcp-Method and Mcp-Name headers" do
    missing_method = rpc("tools/list", modern_meta,
      headers: mcp_headers("MCP-Protocol-Version" => "2026-07-28"))
    assert_response :bad_request
    assert_equal(-32020, missing_method.dig("error", "code"))

    mismatched = rpc("tools/list", modern_meta,
      headers: mcp_headers("MCP-Protocol-Version" => "2026-07-28", "Mcp-Method" => "tools/call"))
    assert_response :bad_request
    assert_equal(-32020, mismatched.dig("error", "code"))

    missing_name = rpc("tools/call", modern_meta.merge("name" => "list_rooms", "arguments" => {}),
      headers: modern_headers("tools/call"))
    assert_response :bad_request
    assert_equal(-32020, missing_name.dig("error", "code"))

    wrong_name = rpc("tools/call", modern_meta.merge("name" => "list_rooms", "arguments" => {}),
      headers: modern_headers("tools/call", "poll_events"))
    assert_response :bad_request
    assert_equal(-32020, wrong_name.dig("error", "code"))
  end

  test "base64 Mcp-Name decodes before comparison" do
    encoded = "=?base64?#{Base64.strict_encode64("list_rooms")}?="
    body = rpc("tools/call", modern_meta.merge("name" => "list_rooms", "arguments" => {}),
      headers: modern_headers("tools/call", encoded))

    assert_response :success
    assert_equal false, body.dig("result", "isError")
  end

  test "unknown methods are 404 with method-not-found" do
    body = rpc("resources/list", {})

    assert_response :not_found
    assert_equal(-32601, body.dig("error", "code"))
  end

  test "unparseable bodies are 400 and malformed envelopes are invalid requests" do
    post agents_mcp_url, params: "{nope", headers: mcp_headers("Content-Type" => "application/json")
    assert_response :bad_request
    assert_equal(-32700, response.parsed_body.dig("error", "code"))
    assert_nil response.parsed_body["id"]

    post agents_mcp_url, params: "{nope", headers: mcp_headers("Content-Type" => "text/plain")
    assert_response :bad_request
    assert_equal(-32700, response.parsed_body.dig("error", "code"))

    post agents_mcp_url, params: { hello: "world" }.to_json, headers: mcp_headers
    assert_response :bad_request
    assert_equal(-32600, response.parsed_body.dig("error", "code"))
  end

  test "batch arrays are rejected as invalid requests with 400" do
    post agents_mcp_url,
      params: [ { jsonrpc: "2.0", id: 1, method: "ping", params: {} } ].to_json,
      headers: mcp_headers

    assert_response :bad_request
    assert_equal(-32600, response.parsed_body.dig("error", "code"))
  end

  test "notifications answer 202 with no body" do
    post agents_mcp_url,
      params: { jsonrpc: "2.0", method: "notifications/initialized" }.to_json,
      headers: mcp_headers

    assert_response :accepted
    assert_empty response.body
  end

  test "GET and DELETE are 405" do
    get agents_mcp_url, headers: mcp_headers
    assert_response :method_not_allowed

    delete agents_mcp_url, headers: mcp_headers
    assert_response :method_not_allowed
  end

  test "mismatched origins are forbidden, matching and missing pass" do
    rpc("ping", {}, headers: mcp_headers("Origin" => "https://evil.example"))
    assert_response :forbidden

    rpc("ping", {}, headers: mcp_headers("Origin" => "http://www.example.com"))
    assert_response :success

    rpc("ping", {})
    assert_response :success
  end

  test "bad tokens are 401 and session requests are 403" do
    post agents_mcp_url,
      params: { jsonrpc: "2.0", id: 1, method: "ping", params: {} }.to_json,
      headers: { "Authorization" => "Bearer wrong-secret", "Content-Type" => "application/json" }
    assert_response :unauthorized

    sign_in :david
    post agents_mcp_url,
      params: { jsonrpc: "2.0", id: 1, method: "ping", params: {} }.to_json,
      headers: { "Content-Type" => "application/json" }
    assert_response :forbidden
  end

  test "ping answers an empty result" do
    assert_equal({}, rpc("ping", {})["result"])
  end

  test "unknown tools and bad arguments are invalid params" do
    unknown = call_tool("launch_missiles", {})
    assert_response :success
    assert_equal(-32602, unknown.dig("error", "code"))

    missing = call_tool("react", { "message_id" => 1 })
    assert_equal(-32602, missing.dig("error", "code"))

    not_object = rpc("tools/call", { "name" => "list_rooms", "arguments" => [ 1 ] })
    assert_equal(-32602, not_object.dig("error", "code"))
  end

  test "list_rooms returns member rooms with kind flags" do
    body = call_tool("list_rooms")

    rooms = structured(body)
    watercooler = rooms.find { |room| room["id"] == @room.id }
    assert_equal "All Talk", watercooler["name"]
    assert_equal "closed", watercooler["type"]
    assert_equal false, watercooler["board"]
    assert_not_includes rooms.map { |room| room["id"] }, rooms(:designers).id
  end

  test "list_rooms returns only rooms with a granted capability" do
    grant!(capability: "read_messages", room: @room)

    ids = structured(call_tool("list_rooms")).map { |room| room["id"] }

    assert_includes ids, @room.id
    assert_not_includes ids, rooms(:bender_and_kevin).id
    assert_not_includes ids, rooms(:designers).id
  end

  test "list_rooms includes every member room with a workspace-wide grant" do
    grant!(capability: "read_messages")

    ids = structured(call_tool("list_rooms")).map { |room| room["id"] }

    assert_includes ids, @room.id
    assert_includes ids, rooms(:bender_and_kevin).id
    assert_not_includes ids, rooms(:designers).id
  end

  test "list_rooms lists nothing for an agent whose only grant is dm_anyone" do
    grant!(capability: "dm_anyone")

    assert_empty structured(call_tool("list_rooms"))
  end

  test "list_rooms is empty when every grant is revoked" do
    grant = AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")
    grant.revoke!

    assert_empty structured(call_tool("list_rooms"))
  end

  test "list_rooms omits soft-deleted rooms" do
    rooms(:bender_and_kevin).update!(deleted_at: Time.current)

    ids = structured(call_tool("list_rooms")).map { |room| room["id"] }

    assert_includes ids, @room.id
    assert_not_includes ids, rooms(:bender_and_kevin).id
  end

  test "read_messages returns a chronological page with cursors" do
    first = @room.messages.create!(creator: users(:david), body: "Read one", client_message_id: "mcp-read-1")
    second = @room.messages.create!(creator: @bot, body: "Read two", client_message_id: "mcp-read-2")

    body = call_tool("read_messages", { "room_id" => @room.id, "after" => first.id })
    page = structured(body)

    assert_equal [ second.id ], page["messages"].map { |message| message["id"] }
    assert_equal second.id, page["after"]
    assert_equal true, page["has_more_before"]
    assert_equal false, page["has_more_after"]
    assert_equal "Read two", page["messages"].first.dig("body", "plain_text")

    before_page = structured(call_tool("read_messages", { "room_id" => @room.id, "before" => second.id, "limit" => 5 }))
    assert_equal 5, before_page["messages"].size
    assert_equal true, before_page["has_more_before"]
    assert_equal true, before_page["has_more_after"]
  end

  test "post_message posts a root message" do
    body = call_tool("post_message", { "room_id" => @room.id, "markdown_source" => "MCP root" })

    message = @room.messages.order(:id).last
    assert_equal "MCP root", message.plain_text_body
    assert_equal message.id, structured(body)["id"]
    assert_nil structured(body)["thread_id"]
  end

  test "post_message replies inside a thread" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "MCP thread")
    ThreadMembership.join!(thread, users(:david))

    body = call_tool("post_message",
      { "room_id" => @room.id, "thread_id" => thread.id, "markdown_source" => "MCP reply" })

    assert_equal thread.id, structured(body)["thread_id"]
    assert_equal "MCP reply", thread.messages.sole.plain_text_body
  end

  test "react is idempotent" do
    message = @room.messages.create!(creator: users(:david), body: "React me", client_message_id: "mcp-react-1")

    first = call_tool("react", { "message_id" => message.id, "content" => "👍" })
    assert_equal true, structured(first)["created"]

    second = call_tool("react", { "message_id" => message.id, "content" => "👍" })
    assert_equal false, structured(second)["created"]
    assert_equal structured(first)["id"], structured(second)["id"]
    assert_equal 1, message.boosts.count
  end

  test "pin_message pins through the shared service" do
    grant!(capability: "post_messages", room: @room)
    message = @room.messages.create!(creator: users(:david), body: "Pin me", client_message_id: "mcp-pin-1")

    body = call_tool("pin_message", { "message_id" => message.id })

    assert_equal({ "pinned" => true, "message_id" => message.id, "pin_count" => 1 }, structured(body))
    assert_equal @bot, @room.message_pins.sole.pinner
    assert_equal @bot, @room.messages.ordered.last.creator
  end

  test "pin_message is idempotent" do
    grant!(capability: "post_messages", room: @room)
    message = @room.messages.create!(creator: users(:david), body: "Pin me twice", client_message_id: "mcp-pin-2")

    call_tool("pin_message", { "message_id" => message.id })

    assert_no_difference -> { MessagePin.count } do
      body = call_tool("pin_message", { "message_id" => message.id })
      assert_equal true, structured(body)["pinned"]
    end
  end

  test "pin_message answers the room cap" do
    grant!(capability: "post_messages", room: @room)
    MessagePin::MAX_PER_ROOM.times do |n|
      message = @room.root_messages.create!(creator: users(:david), markdown_source: "Pinnable #{n}", client_message_id: "mcp-pin-cap-#{n}")
      MessagePin.pin!(message:, pinner: users(:david))
    end
    message = @room.messages.create!(creator: users(:david), body: "One too many", client_message_id: "mcp-pin-cap-last")

    assert_tool_error call_tool("pin_message", { "message_id" => message.id }),
      "This channel already has 50 pinned messages"
  end

  test "unpin_message unpins" do
    grant!(capability: "post_messages", room: @room)
    message = @room.messages.create!(creator: users(:david), body: "Unpin me", client_message_id: "mcp-unpin-1")
    MessagePin.pin!(message:, pinner: users(:david))

    body = call_tool("unpin_message", { "message_id" => message.id })

    assert_equal({ "pinned" => false, "message_id" => message.id, "pin_count" => 0 }, structured(body))
    assert_not MessagePin.pinned?(message)
  end

  test "unpin_message succeeds when the message is not pinned" do
    grant!(capability: "post_messages", room: @room)
    message = @room.messages.create!(creator: users(:david), body: "Never pinned", client_message_id: "mcp-unpin-2")

    body = call_tool("unpin_message", { "message_id" => message.id })

    assert_equal false, structured(body)["pinned"]
  end

  test "poll_events returns rows with a cursor" do
    @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
      client_message_id: "mcp-poll-1"
    )

    body = call_tool("poll_events", { "since" => 0 })
    page = structured(body)

    assert_equal "mention", page["events"].first["event_type"]
    assert page["next_since"] > 0

    follow_up = call_tool("poll_events", { "since" => page["next_since"] })
    assert_empty structured(follow_up)["events"]
  end

  test "ack_events acks rows and reports misses per id" do
    @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
      client_message_id: "mcp-ack-1"
    )
    event_id = @agent.agent_events.deliverable.last.id

    body = call_tool("ack_events", { "event_ids" => [ event_id, 999_999 ] })
    results = structured(body)["results"]

    assert_equal({ "id" => event_id, "outcome" => "acknowledged" }, results.first)
    assert_equal 999_999, results.second["id"]
    assert_equal "Event not found", results.second["error"]
    assert @agent.agent_events.find(event_id).acknowledged?
  end

  test "list_board_posts lists the board" do
    board = create_board!
    grant!(capability: "read_messages", room: board)
    post = ChannelThread.create_board_post!(room: board, creator: users(:david), name: "MCP post", work_status: "planned")

    body = call_tool("list_board_posts", { "room_id" => board.id, "status" => "all" })

    assert_equal [ post.id ], structured(body).map { |row| row["id"] }
  end

  test "create_board_post creates through the shared path" do
    board = create_board!
    grant!(capability: "read_messages", room: board)
    grant!(capability: "post_messages", room: board)
    grant!(capability: "manage_threads", room: board)

    body = call_tool("create_board_post",
      { "room_id" => board.id, "title" => "MCP launch", "body" => "Ships Friday", "tags" => [ "launch" ] })
    payload = structured(body)

    assert_equal "MCP launch", payload["title"]
    assert_equal [ "launch" ], payload["tags"]
    assert_equal @bot.id, payload.dig("owner", "id")
    assert_equal "Ships Friday", ChannelThread.find(payload["id"]).messages.sole.plain_text_body
  end

  test "update_board_post updates status, tags, and run link" do
    board = create_board!
    grant!(capability: "read_messages", room: board)
    grant!(capability: "post_messages", room: board)
    grant!(capability: "manage_threads", room: board)
    post = ChannelThread.create_board_post!(room: board, creator: @bot, name: "MCP update", work_status: "planned", owner_id: @bot.id)

    body = call_tool("update_board_post",
      { "post_id" => post.id, "work_status" => "blocked", "note" => "Waiting", "tags" => "launch,api" })
    payload = structured(body)

    assert_equal "blocked", payload["work_status"]
    assert_equal %w[ api launch ], payload["tags"]
  end

  test "set_result replaces the pinned result" do
    board = create_board!
    grant!(capability: "read_messages", room: board)
    grant!(capability: "post_messages", room: board)
    grant!(capability: "manage_threads", room: board)
    post = ChannelThread.create_board_post!(room: board, creator: @bot, name: "MCP result", work_status: "planned", owner_id: @bot.id)

    body = call_tool("set_result", { "post_id" => post.id, "markdown" => "## Shipped" })

    assert_equal "## Shipped", structured(body)["result"]
  end

  test "set_result checks ownership before markdown presence" do
    board = create_board!
    grant!(capability: "read_messages", room: board)
    grant!(capability: "post_messages", room: board)
    grant!(capability: "manage_threads", room: board)
    foreign = ChannelThread.create_board_post!(room: board, creator: users(:david), name: "Not mine", work_status: "planned", owner_id: users(:david).id)
    owned = ChannelThread.create_board_post!(room: board, creator: @bot, name: "Mine", work_status: "planned", owner_id: @bot.id)

    assert_tool_error call_tool("set_result", { "post_id" => foreign.id }), "Work not found"
    assert_tool_error call_tool("set_result", { "post_id" => owned.id }), "Markdown can't be blank"
  end

  test "list_work lists owned threads" do
    grant!(capability: "read_messages", room: @room)
    grant!(capability: "post_messages", room: @room)
    owned = create_owned_thread!(name: "MCP owned")

    body = call_tool("list_work")

    assert_equal [ owned.id ], structured(body).map { |row| row["id"] }
  end

  test "update_work updates status with a note" do
    grant!(capability: "read_messages", room: @room)
    grant!(capability: "post_messages", room: @room)
    grant!(capability: "manage_threads", room: @room)
    owned = create_owned_thread!(name: "MCP progress")

    body = call_tool("update_work", { "work_id" => owned.id, "work_status" => "in_progress", "note" => "On it" })

    assert_equal "in_progress", structured(body)["work_status"]
  end

  test "request_approval creates and replays on external_id" do
    grant!(capability: "external_action")

    first = call_tool("request_approval",
      { "action" => "deploy", "summary" => "Ship it", "external_id" => "mcp-approval-1" })
    assert_equal "pending", structured(first)["status"]

    second = call_tool("request_approval",
      { "action" => "deploy", "summary" => "Ship it", "external_id" => "mcp-approval-1" })
    assert_equal structured(first)["id"], structured(second)["id"]
    assert_equal 1, @agent.agent_approvals.count
  end

  test "get_approval reads the request with its decision" do
    grant!(capability: "external_action")
    approval = AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")

    body = call_tool("get_approval", { "approval_id" => approval.id })
    payload = structured(body)

    assert_equal "deploy", payload["action"]
    assert_equal "pending", payload["status"]
  end

  test "get_context returns trigger, thread, window, and room" do
    trigger = @room.messages.create!(creator: users(:david), body: "Trigger me", client_message_id: "mcp-ctx-1")

    body = call_tool("get_context", { "message_id" => trigger.id, "limit" => 10 })
    payload = structured(body)

    assert_equal trigger.id, payload.dig("message", "id")
    assert_equal trigger.id, payload["messages"].last["id"]
    assert_equal @room.name, payload.dig("room", "name")
    assert payload["authors"].any? { |author| author["human"] }
  end

  test "get_context answers the same 404 whatever thread_id a foreign room passes" do
    foreign = rooms(:designers).messages.create!(
      creator: users(:david), body: "Stranger", client_message_id: "mcp-ctx-foreign"
    )
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Decoy thread")
    ThreadMembership.join!(thread, users(:david))

    assert_tool_error call_tool("get_context", { "message_id" => foreign.id }),
      "Message not found"
    assert_tool_error call_tool("get_context", { "message_id" => foreign.id, "thread_id" => thread.id }),
      "Message not found"
  end

  test "get_context answers the same 403 whatever thread_id an unreadable room passes" do
    grant!(capability: "read_messages", room: rooms(:bender_and_kevin))
    trigger = @room.messages.create!(creator: users(:david), body: "Unreadable", client_message_id: "mcp-ctx-unreadable")
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Decoy thread")
    ThreadMembership.join!(thread, users(:david))

    assert_tool_error call_tool("get_context", { "message_id" => trigger.id }),
      "Forbidden: agent lacks read_messages capability"
    assert_tool_error call_tool("get_context", { "message_id" => trigger.id, "thread_id" => thread.id }),
      "Forbidden: agent lacks read_messages capability"
  end

  test "get_context still rejects mismatched threads for authorized callers" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Other thread")
    ThreadMembership.join!(thread, users(:david))
    trigger = @room.messages.create!(creator: users(:david), body: "Root trigger", client_message_id: "mcp-ctx-mismatch")

    assert_tool_error call_tool("get_context", { "message_id" => trigger.id, "thread_id" => thread.id }),
      "Message is not in the given thread"
  end

  test "open_dm opens the owner DM and posts" do
    body = call_tool("open_dm", { "user_id" => users(:david).id, "markdown_source" => "MCP DM hello" })
    payload = structured(body)

    room = Room.find(payload.dig("room", "id"))
    assert_equal [ @bot.id, users(:david).id ].sort, room.user_ids.sort
    assert_equal "MCP DM hello", payload.dig("message", "body", "plain_text")
  end

  test "post_message denies without post_messages" do
    grant!(capability: "read_messages", room: @room)

    body = call_tool("post_message", { "room_id" => @room.id, "markdown_source" => "Denied" })

    assert_tool_error body, "Forbidden: agent lacks post_messages capability"
  end

  test "read tools deny without read_messages" do
    grant!(capability: "post_messages", room: @room)
    trigger = @room.messages.create!(creator: users(:david), body: "Gated", client_message_id: "mcp-gated-1")

    assert_tool_error call_tool("read_messages", { "room_id" => @room.id }),
      "Forbidden: agent lacks read_messages capability"
    assert_tool_error call_tool("poll_events", {}),
      "Forbidden: agent lacks read_messages capability"
    assert_tool_error call_tool("get_context", { "message_id" => trigger.id }),
      "Forbidden: agent lacks read_messages capability"
    assert_tool_error call_tool("list_board_posts", { "room_id" => @room.id }),
      "Forbidden: agent lacks read_messages capability"
  end

  test "react denies without react" do
    grant!(capability: "read_messages", room: @room)
    grant!(capability: "post_messages", room: @room)
    message = @room.messages.create!(creator: users(:david), body: "No react", client_message_id: "mcp-gated-2")

    assert_tool_error call_tool("react", { "message_id" => message.id, "content" => "👍" }),
      "Forbidden: agent lacks react capability"
  end

  test "pin tools deny without post_messages" do
    grant!(capability: "read_messages", room: @room)
    message = @room.messages.create!(creator: users(:david), body: "No pin", client_message_id: "mcp-gated-3")

    assert_tool_error call_tool("pin_message", { "message_id" => message.id }),
      "Forbidden: agent lacks post_messages capability"
    assert_tool_error call_tool("unpin_message", { "message_id" => message.id }),
      "Forbidden: agent lacks post_messages capability"
  end

  test "pin tools are not found outside the agent's memberships" do
    grant!(capability: "post_messages")

    pin_body = call_tool("pin_message", { "message_id" => messages(:first).id })
    assert_tool_error pin_body, "Message not found"
    assert_equal "not_found", pin_body.dig("result", "structuredContent", "status")

    unpin_body = call_tool("unpin_message", { "message_id" => messages(:first).id })
    assert_tool_error unpin_body, "Message not found"
    assert_equal "not_found", unpin_body.dig("result", "structuredContent", "status")
  end

  test "pin tools require a message id" do
    pin_body = call_tool("pin_message", {})

    assert_response :success
    assert_equal(-32602, pin_body.dig("error", "code"))
    assert_equal "Missing required argument: message_id", pin_body.dig("error", "message")

    unpin_body = call_tool("unpin_message", {})

    assert_equal(-32602, unpin_body.dig("error", "code"))
  end

  test "board and work writes deny without manage_threads" do
    board = create_board!
    grant!(capability: "read_messages", room: board)
    grant!(capability: "post_messages", room: board)
    grant!(capability: "read_messages", room: @room)
    grant!(capability: "post_messages", room: @room)

    assert_tool_error call_tool("create_board_post", { "room_id" => board.id, "title" => "Denied" }),
      "Forbidden: agent lacks manage_threads capability"

    owned = create_owned_thread!(name: "MCP gated work")
    assert_tool_error call_tool("update_work", { "work_id" => owned.id, "work_status" => "done" }),
      "Forbidden: agent lacks manage_threads capability"
    assert_tool_error call_tool("update_board_post", { "post_id" => owned.id, "work_status" => "done" }),
      "Forbidden: agent lacks manage_threads capability"
    assert_tool_error call_tool("set_result", { "post_id" => owned.id, "markdown" => "Denied" }),
      "Forbidden: agent lacks manage_threads capability"
  end

  test "approval tools deny without external_action" do
    grant!(capability: "read_messages", room: @room)
    approval = AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")

    assert_tool_error call_tool("request_approval", { "action" => "deploy", "summary" => "Ship it" }),
      "Forbidden: agent lacks external_action capability"
    assert_tool_error call_tool("get_approval", { "approval_id" => approval.id }),
      "Forbidden: agent lacks external_action capability"
  end

  test "open_dm denies strangers without dm_anyone" do
    assert_tool_error call_tool("open_dm", { "user_id" => users(:jason).id, "markdown_source" => "Denied" }),
      "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability"
  end

  test "open_dm ignores a room-scoped dm_anyone grant" do
    AgentGrant.new(agent: @agent, room: @room, granted_by: users(:david), capability: "dm_anyone")
      .save!(validate: false)
    grant!(capability: "post_messages")

    assert_tool_error call_tool("open_dm", { "user_id" => users(:jason).id, "markdown_source" => "Denied" }),
      "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability"
  end

  test "open_dm denies its owner when all grants are revoked" do
    grant = AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")
    grant.revoke!

    assert_tool_error call_tool("open_dm", { "user_id" => users(:david).id, "markdown_source" => "Denied" }),
      "Forbidden: agent lacks post_messages capability"
  end

  test "open_dm denies read-only agents" do
    grant!(capability: "read_messages")

    assert_tool_error call_tool("open_dm", { "user_id" => users(:david).id, "markdown_source" => "Denied" }),
      "Forbidden: agent lacks post_messages capability"
  end

  test "open_dm denies an existing DM where posting was revoked" do
    grant = AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    created = call_tool("open_dm", { "user_id" => users(:david).id, "markdown_source" => "One" })
    assert_equal false, created.dig("result", "isError")

    grant.revoke!
    grant!(capability: "post_messages", room: @room)

    assert_tool_error call_tool("open_dm", { "user_id" => users(:david).id, "markdown_source" => "Two" }),
      "Forbidden: agent lacks post_messages capability"
  end

  test "ack_events rejects more than 100 ids" do
    body = call_tool("ack_events", { "event_ids" => (1..101).to_a })

    assert_response :success
    assert_equal(-32602, body.dig("error", "code"))
    assert_equal "event_ids must contain at most 100 ids", body.dig("error", "message")
  end

  test "ack_events charges the ack bucket once per id" do
    with_memory_cache do
      freeze_time do
        first = call_tool("ack_events", { "event_ids" => (1..100).to_a })
        assert_equal false, first.dig("result", "isError")

        second = call_tool("ack_events", { "event_ids" => (101..120).to_a })
        assert_equal false, second.dig("result", "isError")

        body = call_tool("ack_events", { "event_ids" => [ 999_999 ] })

        assert_response :success
        assert_equal true, body.dig("result", "isError")
        assert_equal "rate_limited", body.dig("result", "structuredContent", "error")
        assert response.headers["Retry-After"].present?
      end
    end
  end

  test "ack_events per-id charges drain the shared REST ack bucket" do
    with_memory_cache do
      freeze_time do
        call_tool("ack_events", { "event_ids" => (1..100).to_a })
        call_tool("ack_events", { "event_ids" => (101..120).to_a })

        post ack_agents_event_url(999_999), headers: mcp_headers

        assert_response :too_many_requests
        assert_equal "rate_limited", response.parsed_body["error"]
      end
    end
  end

  test "poll_events throttles past 120 calls a minute" do
    with_memory_cache do
      freeze_time do
        120.times { call_tool("poll_events", {}) }
        assert_equal false, response.parsed_body.dig("result", "isError")

        body = call_tool("poll_events", {})

        assert_response :success
        assert_equal true, body.dig("result", "isError")
        assert_match "rate_limited", body.dig("result", "content").first["text"]
        assert_equal "rate_limited", body.dig("result", "structuredContent", "error")
        assert response.headers["Retry-After"].present?
      end
    end
  end

  test "MCP tools share buckets with their REST endpoints" do
    board = create_board!
    grant!(capability: "read_messages", room: board)
    grant!(capability: "post_messages", room: board)
    grant!(capability: "manage_threads", room: board)

    with_memory_cache do
      freeze_time do
        30.times do |index|
          post room_agent_posts_url(board), params: { title: "Bucket #{index}" }.to_json, headers: mcp_headers
          assert_response :created
        end

        body = call_tool("create_board_post", { "room_id" => board.id, "title" => "Over the bucket" })

        assert_equal true, body.dig("result", "isError")
        assert_equal "rate_limited", body.dig("result", "structuredContent", "error")
      end
    end
  end

  test "pin_message shares its bucket with the REST pin endpoint" do
    grant!(capability: "post_messages", room: @room)
    message = @room.messages.create!(creator: users(:david), body: "Bucket pin", client_message_id: "mcp-pin-bucket-1")

    with_memory_cache do
      freeze_time do
        60.times do
          post agents_message_pin_url(message), headers: mcp_headers
          assert_response :success
        end

        body = call_tool("pin_message", { "message_id" => message.id })

        assert_equal true, body.dig("result", "isError")
        assert_equal "rate_limited", body.dig("result", "structuredContent", "error")
      end
    end
  end

  test "the endpoint throttles past 600 requests a minute across all methods" do
    _, second_secret = AgentCredential.create_with_secret!(agent: @agent, name: "second", created_by: users(:david))

    with_memory_cache do
      freeze_time do
        200.times { |index| rpc("ping", {}, id: index) }
        200.times { |index| rpc("tools/list", {}, id: index) }
        200.times do |index|
          rpc("initialize", { "protocolVersion" => "2025-11-25", "capabilities" => {}, "clientInfo" => { "name" => "c", "version" => "1" } }, id: index)
        end
        assert_response :success

        rpc("ping", {})
        assert_response :too_many_requests
        assert_equal "rate_limited", response.parsed_body["error"]
        retry_after = response.headers["Retry-After"].to_i
        assert_operator retry_after, :>=, 1
        assert_operator retry_after, :<=, 60

        post agents_mcp_url,
          params: { jsonrpc: "2.0", id: 1, method: "ping", params: {} }.to_json,
          headers: { "Authorization" => "Bearer #{second_secret}", "Content-Type" => "application/json" }
        assert_response :success
      end
    end
  end

  test "get_context shares its bucket with the REST context endpoint" do
    trigger = @room.messages.create!(creator: users(:david), body: "Shared bucket", client_message_id: "mcp-ctx-share")

    with_memory_cache do
      freeze_time do
        120.times do
          get agents_context_url(message_id: trigger.id), headers: mcp_headers
          assert_response :success
        end

        body = call_tool("get_context", { "message_id" => trigger.id })

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

    def modern_headers(method, name = nil)
      headers = mcp_headers("MCP-Protocol-Version" => "2026-07-28", "Mcp-Method" => method)
      headers["Mcp-Name"] = name if name

      headers
    end

    def modern_meta(version = "2026-07-28")
      { "_meta" => { "io.modelcontextprotocol/protocolVersion" => version } }
    end

    def grant!(capability:, room: nil)
      AgentGrant.create!(agent: @agent, room: room, granted_by: users(:david), capability: capability)
    end

    def create_board!(name: "Launch")
      board = Rooms::Board.create_for({ name: name, creator: users(:david) }, users: [ users(:david) ])
      board.memberships.grant_to(@bot)

      board
    end

    def create_owned_thread!(name:)
      thread = ChannelThread.create!(room: @room, creator: users(:david), name: name)
      ThreadMembership.join!(thread, users(:david))
      thread.update_work!(actor: users(:david), work_status: "planned", work_owner_id: @bot.id)

      thread
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
