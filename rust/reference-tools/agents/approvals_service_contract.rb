require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  actor = User.find(127326141)
  agent.agent_approvals.destroy_all
  agent.agent_grants.delete_all
  AgentApproval.connection.execute("DELETE FROM sqlite_sequence WHERE name='agent_approvals'")
  AgentApproval.connection.execute("INSERT INTO sqlite_sequence(name,seq) VALUES ('agent_approvals',900030000)")
  agent.update_columns(daily_external_action_cap: nil)
  results = {}
  capture = lambda do |name, result|
    code = result.status == :unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status)
    results[name] = { status: code, error: result.error, payload: result.payload }
    result
  end
  fields = { "action" => "deploy", "summary" => "Ship", "payload" => { "key" => "<>&" }, "external_id" => "repeat" }
  capture.call(:forbidden, Agents::Approvals.create(agent: agent, fields: fields))
  capture.call(:missing_before_grant, Agents::Approvals.create(agent: agent, fields: fields.merge("room_id" => 0)))
  capture.call(:show_missing, Agents::Approvals.show(agent: agent, id: 0))
  grant = AgentGrant.create!(agent: agent, granted_by: actor, capability: "external_action")
  created = capture.call(:created, Agents::Approvals.create(agent: agent, fields: fields))
  approval = AgentApproval.find(created.payload[:id])
  agent.update_columns(daily_external_action_cap: 1)
  capture.call(:replay, Agents::Approvals.create(agent: agent, fields: fields.merge("action" => "github.comment")))
  capture.call(:github_blocked, Agents::Approvals.create(agent: agent, fields: fields.except("external_id").merge("action" => "github.comment")))
  capture.call(:fizzy_blocked, Agents::Approvals.create(agent: agent, fields: fields.except("external_id").merge("action" => "fizzy.close")))
  capture.call(:budget, Agents::Approvals.create(agent: agent, fields: fields.except("external_id")))
  capture.call(:show, Agents::Approvals.show(agent: agent, id: approval.id))
  capture.call(:cancel, Agents::Approvals.cancel(agent: agent, id: approval.id))
  capture.call(:cancel_again, Agents::Approvals.cancel(agent: agent, id: approval.id))
  agent.update_columns(daily_external_action_cap: nil)
  capture.call(:invalid, Agents::Approvals.create(agent: agent, fields: { "action" => "", "summary" => "" }))
  overdue = AgentApproval.create!(agent: agent, action: "deploy", summary: "Overdue")
  overdue.update_columns(expires_at: Time.current)
  fresh = AgentApproval.create!(agent: agent, action: "deploy", summary: "Fresh")
  capture.call(:pending, Agents::Approvals.list(agent: agent, status: "pending"))
  capture.call(:expired, Agents::Approvals.list(agent: agent, status: "expired"))
  capture.call(:all, Agents::Approvals.list(agent: agent, status: "unknown"))
  grant.revoke!
  capture.call(:revoked_show, Agents::Approvals.show(agent: agent, id: approval.id))
  capture.call(:revoked_list, Agents::Approvals.list(agent: agent))
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", results: results }.as_json)
end
