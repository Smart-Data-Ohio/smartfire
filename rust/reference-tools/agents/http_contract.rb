# HTTP oracle for WS11-api. Run in the pinned production reference over default.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers

travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  agent.agent_events.delete_all
  agent.agent_grants.delete_all
  agent.agent_credentials.delete_all
  secret = "ws11api-fixture-credential"
  agent.agent_credentials.create!(name: "HTTP contract", created_by: User.find(127326141), token_digest: AgentCredential.digest(secret), token_last_four: AgentCredential.digest(secret).first(4))
  agent.agent_events.create!(id: 900_000_001, event_type: "github_action_completed", outcome: "delivered", metadata: { action: "github.comment", status: "done" })
  agent.agent_events.create!(id: 900_000_002, event_type: "approval_decided", outcome: "delivered", metadata: { approval_id: 0 })
  Rails.cache = ActiveSupport::Cache::MemoryStore.new
  cases = []
  capture = lambda do |name, method, path, body = nil, headers = {}, token = secret|
    session = ActionDispatch::Integration::Session.new(Rails.application)
    session.host! "campfire.test"
    request_headers = { "Accept" => "application/json", "Content-Type" => "application/json" }.merge(headers)
    request_headers["Authorization"] = ["Bearer", token].join(" ") if token
    raw = body.is_a?(String) ? body : body&.to_json
    session.public_send(method, path, params: raw, headers: request_headers)
    response = session.response
    decoded = response.body.blank? ? nil : JSON.parse(response.body)
    cases << { name: name, method: method, path: path, body: raw, headers: headers, token: token,
      status: response.status, response: decoded,
      response_headers: response.headers.slice("Cache-Control", "Pragma", "Retry-After", "X-Smartfire-Next-Since") }
  end
  capture.call("event_envelope", :get, "/agents/events?envelope=1")
  capture.call("event_array", :get, "/agents/events")
  capture.call("event_dropped_cursor", :get, "/agents/events?since=900000001&limit=1&envelope=1")
  capture.call("event_ack", :post, "/agents/events/900000001/ack", {})
  capture.call("event_ack_replay", :post, "/agents/events/900000001/ack", {})
  capture.call("event_ack_missing", :post, "/agents/events/0/ack", {})
  capture.call("agent_human_endpoint", :get, "/rooms.json")
  capture.call("unknown_credential", :get, "/agents/events", nil, {}, "ws11api-unknown")
  credential = agent.agent_credentials.first
  credential.update!(revoked_at: Time.current)
  capture.call("revoked_credential", :get, "/agents/events")
  credential.update!(revoked_at: nil, expires_at: Time.current)
  capture.call("expired_credential", :get, "/agents/events")
  credential.update!(expires_at: nil)
  AgentGrant.create!(agent: agent, capability: "read_messages", granted_by: User.find(127326141), revoked_at: Time.current)
  capture.call("missing_grant", :get, "/agents/events")
  agent.agent_grants.delete_all
  capture.call("step_no_parent", :post, "/agents/steps", { name: "Inspect" })
  capture.call("step_missing", :patch, "/agents/steps/0", { name: "Inspect" })
  capture.call("slash_nonmember", :post, "/rooms/201306877/agents/slash_commands", { name: "inspect" })
  AgentGrant.create!(agent: agent, capability: "read_messages", granted_by: User.find(127326141))
  capture.call("slash_missing_grant", :post, "/rooms/486777696/agents/slash_commands", { name: "inspect" })
  agent.agent_grants.delete_all
  capture.call("slash_invalid", :post, "/rooms/486777696/agents/slash_commands", { name: "123" })
  capture.call("slash_missing", :delete, "/rooms/486777696/agents/slash_commands/absent")
  # One credential and one minute bucket; failed requests count too.
  Rails.cache.clear
  120.times do
    s = ActionDispatch::Integration::Session.new(Rails.application)
    s.host! "campfire.test"
    s.get "/agents/events", headers: { "Authorization" => ["Bearer", secret].join(" "), "Accept" => "application/json" }
  end
  capture.call("event_rate_overflow", :get, "/agents/events")
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", cases: cases })
end
