# Protocol metadata and actual request/response pairs from our pinned Rails app.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  agent.agent_events.delete_all
  agent.agent_grants.delete_all
  agent.agent_credentials.delete_all
  secret = "ws11api-fixture-credential"
  agent.agent_credentials.create!(name: "MCP contract", created_by: User.find(127326141), token_digest: AgentCredential.digest(secret), token_last_four: AgentCredential.digest(secret).first(4))
  Rails.cache = ActiveSupport::Cache::MemoryStore.new
  cases = []
  capture = lambda do |name, envelope, headers = {}, method = :post|
    session = ActionDispatch::Integration::Session.new(Rails.application)
    session.host! "campfire.test"
    raw = envelope.is_a?(String) ? envelope : JSON.generate(envelope)
    session.public_send(method, "/agents/mcp", params: raw,
      headers: { "Authorization" => ["Bearer", secret].join(" "), "Accept" => "application/json", "Content-Type" => "application/json" }.merge(headers))
    response = session.response
    cases << { name: name, method: method, body: raw, headers: headers, status: response.status,
      response: response.body.blank? ? nil : JSON.parse(response.body),
      response_headers: response.headers.slice("Cache-Control", "Pragma", "Retry-After") }
  end
  rpc = lambda { |method, params = {}| { jsonrpc: "2.0", id: 7, method: method, params: params } }
  versions = Agents::McpServer::SUPPORTED_VERSIONS
  versions.each do |version|
    capture.call("initialize_#{version}", rpc.call("initialize", { protocolVersion: version }), version == versions.first ? { "Mcp-Method" => "initialize" } : {})
    capture.call("ping_#{version}", rpc.call("ping"), { "MCP-Protocol-Version" => version }.merge(version == versions.first ? { "Mcp-Method" => "ping" } : {}))
  end
  capture.call("discover_modern", rpc.call("server/discover"), { "MCP-Protocol-Version" => versions.first, "Mcp-Method" => "server/discover" })
  capture.call("discover_legacy", rpc.call("server/discover"))
  capture.call("tools_list", rpc.call("tools/list"))
  capture.call("get_405", {}, {}, :get)
  capture.call("delete_405", {}, {}, :delete)
  capture.call("parse_error", "{")
  capture.call("text_parse_error", "{", { "Content-Type" => "text/plain" })
  capture.call("batch", [])
  capture.call("null", nil)
  capture.call("missing_jsonrpc", { id: 7, method: "ping" })
  capture.call("wrong_method_type", { jsonrpc: "2.0", id: 7, method: 1 })
  capture.call("notification", { jsonrpc: "2.0", method: "notifications/initialized" }, { "MCP-Protocol-Version" => "wrong" })
  capture.call("identified_notification", rpc.call("notifications/initialized"))
  capture.call("unknown_method", rpc.call("unknown"))
  capture.call("unknown_tool", rpc.call("tools/call", { name: "unknown" }))
  capture.call("bad_arguments", rpc.call("tools/call", { name: "poll_events", arguments: [] }))
  capture.call("unsupported", rpc.call("ping"), { "MCP-Protocol-Version" => "2099-01-01" })
  capture.call("unsupported_meta_type", rpc.call("ping", { _meta: { "io.modelcontextprotocol/protocolVersion" => 5 } }))
  capture.call("version_mismatch", rpc.call("ping", { _meta: { "io.modelcontextprotocol/protocolVersion" => "2025-03-26" } }), { "MCP-Protocol-Version" => versions.first })
  capture.call("missing_method_header", rpc.call("ping"), { "MCP-Protocol-Version" => versions.first })
  capture.call("method_mismatch", rpc.call("ping"), { "MCP-Protocol-Version" => versions.first, "Mcp-Method" => "tools/list" })
  modern = { "MCP-Protocol-Version" => versions.first, "Mcp-Method" => "tools/call" }
  capture.call("missing_name", rpc.call("tools/call", { name: "poll_events" }), modern)
  capture.call("name_mismatch", rpc.call("tools/call", { name: "poll_events" }), modern.merge("Mcp-Name" => "ack_events"))
  capture.call("base64_name", rpc.call("tools/call", { name: "poll_events" }), modern.merge("Mcp-Name" => "=?base64?#{Base64.strict_encode64('poll_events')}?="))
  capture.call("bad_base64", rpc.call("tools/call", { name: "poll_events" }), modern.merge("Mcp-Name" => "=?base64?!!!!?="))
  capture.call("bad_utf8_base64", rpc.call("tools/call", { name: "poll_events" }), modern.merge("Mcp-Name" => "=?base64?/w==?="))
  capture.call("origin_denied", rpc.call("ping"), { "Origin" => "https://attacker.invalid" })
  capture.call("origin_allowed", rpc.call("ping"), { "Origin" => "https://CAMPFIRE.test:9443" })
  capture.call("origin_malformed", rpc.call("ping"), { "Origin" => ":invalid" })
  capture.call("origin_encoded_host", rpc.call("ping"), { "Origin" => "https://%63ampfire.test" })
  capture.call("poll_empty", rpc.call("tools/call", { name: "poll_events", arguments: { since: 0, limit: 1 } }))
  capture.call("ack_empty", rpc.call("tools/call", { name: "ack_events", arguments: { event_ids: [] } }))
  capture.call("ack_too_many", rpc.call("tools/call", { name: "ack_events", arguments: { event_ids: Array.new(101, 0) } }))
  capture.call("ack_missing", rpc.call("tools/call", { name: "ack_events", arguments: { event_ids: [0] } }))
  # No malformed domain object is mocked: Ruby naturally raises NoMethodError on to_i.
  capture.call("internal_error", rpc.call("tools/call", { name: "poll_events", arguments: { since: { odd: "shape" } } }))
  Agents::McpServer.tools.each do |tool|
    capture.call("tool_empty_#{tool.name}", rpc.call("tools/call", { name: tool.name, arguments: {} }))
  end
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", versions: versions, server_name: Agents::McpServer::SERVER_NAME,
    server_version: Agents::McpServer::SERVER_VERSION, instructions: Agents::McpServer::INSTRUCTIONS,
    tools: Agents::McpServer.tools.map { |t| { name: t.name, description: t.description, inputSchema: t.input_schema, throttle: t.throttle } }, cases: cases })
end
