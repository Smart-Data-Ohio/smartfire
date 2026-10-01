require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  owner = User.find_by!(name: "Kevin")
  admin = User.find_by!(name: "David")
  agent.update_columns(owner_id: owner.id)
  agent.agent_approvals.destroy_all
  agent.agent_events.delete_all
  agent.agent_grants.delete_all
  base = { agent: agent, action: "deploy", summary: "Ship" }
  validation = {}
  {
    blank: { action: "", summary: "" }, invalid_action: { action: "Deploy!" },
    action_long: { action: "a" * 61 }, summary_long: { summary: "é" * 501 },
    valid_action: { action: "deploy.prod_1-ok" }, invalid_status: { status: "bad" },
    blank_status: { status: "" }, payload_large: { payload: "é" * 2049 },
    payload_boundary: { payload: "é" * 2048 }, note_large: { decision_note: "é" * 201 },
    minimum: { expires_at: Time.current + 4.minutes }, too_soon: { expires_at: Time.current + 4.minutes - 1.second },
    maximum: { expires_at: Time.current + 7.days + 1.minute }, too_late: { expires_at: Time.current + 7.days + 1.minute + 1.second },
    missing_agent: { agent: nil }, optional_missing: { room_id: -1, agent_credential_id: -1, decided_by_id: -1 }
  }.each do |name, fields|
    record = AgentApproval.new(base.merge(fields)); record.valid?
    validation[name] = record.errors.to_hash
  end
  AgentApproval.create!(base.merge(id: 900_020_001, external_id: "repeat"))
  { duplicate: "repeat", blank_external: " ", nil_external: nil }.each do |name, external|
    record = AgentApproval.new(base.merge(external_id: external)); record.valid?
    validation[name] = record.errors.to_hash
  end
  record = AgentApproval.create!(base.merge(id: 900_020_002, room_id: 486777696, payload: '{"key":"<>&"}', external_id: "stable"))
  actors = %w[Kevin David Jason Bender\ Bot JZ].filter_map { |name| User.find_by(name: name) }
  permissions = {}
  %w[deploy github.comment fizzy.close].each do |action|
    record.update_columns(action: action)
    permissions[action] = actors.to_h { |user| [user.name, { decidable: record.decidable_by?(user), approvable: record.approvable_by?(user) }] }
  end
  record.update_columns(action: "deploy")
  inbox = ->(approval) { approval.activity_items.order(:user_id).map { |item| { user_id: item.user_id, unread: item.unread?, handled: item.handled_at.present?, read_at: item.read_at } } }
  flows = { created: Agents::Approvals.approval_payload(record), inbox: inbox.call(record), recipients: record.deciders.map(&:name).sort }
  capture = lambda do |operation|
    begin
      operation.call
      []
    rescue ActiveRecord::RecordInvalid => error
      error.record.errors.to_hash
    rescue ArgumentError => error
      error.message
    end
  end
  stale = AgentApproval.find(record.id)
  adapter = ApplicationJob.queue_adapter
  adapter.enqueued_jobs.clear
  flows[:denied_errors] = capture.call(-> { record.decide!(decision: "denied", by: owner, note: "  ") })
  flows[:denied] = Agents::Approvals.approval_payload(record)
  flows[:settled_inbox] = inbox.call(record)
  flows[:stale_errors] = capture.call(-> { stale.decide!(decision: "approved", by: admin) })
  flows[:events] = agent.agent_events.where(event_type: "approval_decided").map { |event| { agent_approval_id: event.agent_approval_id, outcome: event.outcome, webhook_status: event.webhook_status, metadata: event.metadata } }
  flows[:jobs] = adapter.enqueued_jobs.map { |job| { class: job[:job].name, args: job[:args] } }
  flows[:unknown] = capture.call(-> { record.decide!(decision: "surprise", by: admin) })
  expired = AgentApproval.create!(base.merge(id: 900_020_003))
  expired.update_columns(expires_at: Time.current)
  flows[:effective] = { status: expired.status, effective: expired.effective_status }
  flows[:expired_errors] = capture.call(-> { expired.cancel_by_agent! })
  flows[:expired] = { status: expired.reload.status, inbox: inbox.call(expired) }
  cancelled = AgentApproval.create!(base.merge(id: 900_020_004))
  flows[:cancel_errors] = capture.call(-> { cancelled.cancel_by_agent! })
  flows[:cancel_again] = capture.call(-> { cancelled.cancel_by_agent! })
  flows[:cancel_events] = agent.agent_events.where(agent_approval_id: cancelled.id).count
  note = AgentApproval.create!(base.merge(id: 900_020_005))
  flows[:long_note] = capture.call(-> { note.decide!(decision: "denied", by: admin, note: "é" * 201) })
  flows[:long_note_status] = note.reload.status
  owner.update_columns(inbox_preferences: { agent_approvals: "0" })
  agent.reload
  opted = AgentApproval.create!(base.merge(id: 900_020_006))
  flows[:opt_out] = inbox.call(opted)
  flows[:opt_out_deciders] = opted.deciders.map(&:name).sort
  external = AgentApproval.create!(base.merge(id: 900_020_007, action: "github.comment"))
  adapter.enqueued_jobs.clear
  external.decide!(decision: "approved", by: admin, note: "ok")
  flows[:external_jobs] = adapter.enqueued_jobs.map { |job| { class: job[:job].name, args: job[:args] } }
  agent.update_columns(suspended_at: Time.current)
  flows[:suspended_decidable] = external.decidable_by?(owner)
  agent.user.update_column(:status, :banned)
  flows[:inactive_decidable] = external.reload.decidable_by?(owner)
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", validation: validation, permissions: permissions, flows: flows }.as_json)
end
