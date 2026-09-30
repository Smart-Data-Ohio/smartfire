# Actual Rails REST/MCP requests. Case setup is replayed against the seeded Rust app.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
SECRET = "ws11api-fixture-credential"
CASES = []
def contract(name, method, path, body = nil, setup = {})
  setup = { grant: "external_action" }.merge(setup) if path.include?("approvals") || (body.is_a?(Hash) && %w[request_approval get_approval].include?(body.dig(:params, :name)))
  CASES << { name: name, method: method, path: path, body: body, setup: setup }
end
def tool(name, arguments = {}, setup = {}, label = name)
  contract("mcp_#{label}", :post, "/agents/mcp", { jsonrpc: "2.0", id: 7, method: "tools/call", params: { name: name, arguments: arguments } }, setup)
end
contract("approvals_list", :get, "/agents/approvals")
contract("approvals_show", :get, "/agents/approvals/900100001", nil, { approval: "pending" })
contract("approvals_expired", :get, "/agents/approvals/900100001", nil, { approval: "expired" })
contract("approvals_cancel", :delete, "/agents/approvals/900100001", nil, { approval: "pending" })
contract("approvals_cancel_decided", :delete, "/agents/approvals/900100001", nil, { approval: "approved" })
contract("approvals_missing", :get, "/agents/approvals/0")
contract("approvals_nested", :post, "/agents/approvals", { approval: { action: "deploy", summary: "Ship", payload: { x: [1, true] }, room_id: 486777696, expires_in_seconds: 3600 } })
contract("approvals_top", :post, "/agents/approvals", { action: "deploy", summary: "Ship", expires_at: "2026-03-03T16:00:00Z" })
contract("approvals_invalid", :post, "/agents/approvals", { action: "Bad Action!", summary: "" })
contract("approvals_invalid_expiry", :post, "/agents/approvals", { action: "deploy", summary: "Ship", expires_in: "abc" })
contract("approvals_github", :post, "/agents/approvals", { action: "github.comment", summary: "Ship" })
contract("approvals_fizzy", :post, "/agents/approvals", { action: "fizzy.create", summary: "Ship" })
contract("approvals_nonmember", :post, "/agents/approvals", { action: "deploy", summary: "Ship", room_id: 201306877 })
contract("approvals_denied", :post, "/agents/approvals", { action: "deploy", summary: "Ship" }, { grant: "read_messages" })
contract("approvals_replay", :post, "/agents/approvals", { action: "github.comment", summary: "Ignored", external_id: "surface-replay", expires_in: "abc" }, { approval: "pending", cap: 0 })
contract("approvals_budget", :post, "/agents/approvals", { action: "deploy", summary: "Ship" }, { cap: 0 })
contract("approvals_rate", :get, "/agents/approvals", nil, { repeat: 120 })
contract("approvals_create_rate", :post, "/agents/approvals", {}, { repeat: 60 })
tool("request_approval", { action: "deploy", summary: "Ship", payload: { x: 1 } })
tool("request_approval", {}, {}, "request_approval_invalid")
tool("get_approval", { approval_id: 900100001 }, { approval: "pending" })
tool("get_approval", { approval_id: 0 }, {}, "get_approval_missing")
tool("request_approval", { action: "deploy", summary: "Ship" }, { grant: "read_messages" }, "request_approval_denied")
tool("request_approval", { action: "deploy", summary: "Ship" }, { cap: 0 }, "request_approval_budget")
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  result = CASES.map do |item|
    agent.agent_approvals.delete_all
    AgentApproval.connection.execute("DELETE FROM sqlite_sequence WHERE name='agent_approvals'")
    agent.agent_grants.delete_all
    agent.agent_credentials.delete_all
    agent.update_columns(daily_external_action_cap: item[:setup][:cap], daily_message_cap: nil, daily_board_post_cap: nil)
    credential = agent.agent_credentials.create!(name: "Surface contract", created_by: User.find(127326141), token_digest: AgentCredential.digest(SECRET), token_last_four: AgentCredential.digest(SECRET).first(4))
    if item[:setup][:grant]
      AgentGrant.create!(agent: agent, capability: item[:setup][:grant], granted_by: User.find(127326141))
    end
    if (status = item[:setup][:approval])
      approval = agent.agent_approvals.create!(id: 900100001, action: "deploy", summary: "Existing", room_id: 486777696, external_id: "surface-replay")
      approval.update_columns(status: status == "expired" ? "pending" : status, expires_at: status == "expired" ? 1.minute.ago : 24.hours.from_now)
    end
    Rails.cache = ActiveSupport::Cache::MemoryStore.new
    session = ActionDispatch::Integration::Session.new(Rails.application)
    session.host! "campfire.test"
    headers = { "Accept" => "application/json", "Content-Type" => "application/json", "Authorization" => ["Bearer", SECRET].join(" ") }
    raw = item[:body]&.to_json
    (item[:setup][:repeat] || 0).times { session.public_send(item[:method], item[:path], params: raw, headers: headers) }
    session.public_send(item[:method], item[:path], params: raw, headers: headers)
    response = session.response
    item.merge(body: raw, status: response.status, response: response.body.blank? ? nil : JSON.parse(response.body), response_headers: response.headers.slice("Cache-Control", "Pragma", "Retry-After"))
  end
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", cases: result })
end
