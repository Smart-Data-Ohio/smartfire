require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  bot = agent.user
  human = User.find(127326141)
  room = Room.find(486777696)
  room.memberships.grant_to([bot])
  AgentGrant.delete_all
  AgentEvent.delete_all
  thread = ChannelThread.create!(id: 900070001, room: room, creator: human, name: "Work events", work_status: "in_progress")
  thread.update_columns(work_owner_id: bot.id)
  serialize = ->(events) do
    events.map do |e|
      e.reload
      {agent_id: e.agent_id, event_type: e.event_type, outcome: e.outcome, actor_id: e.actor_id,
       metadata: e.metadata, hop: e.hop, detail: e.detail, webhook_status: e.webhook_status,
       webhook_next_attempt_at: e.webhook_next_attempt_at, chain: e.chain_id == "ws11-trigger-chain" ? e.chain_id : "uuid"}
    end
  end
  results = {}
  results[:same] = serialize.call(thread.send(:record_work_assignment_events!, from_owner: bot, to_owner: bot, actor: human))
  events = thread.send(:record_work_assignment_events!, from_owner: nil, to_owner: bot, actor: human)
  thread.send(:deliver_work_assignment_webhooks, events)
  results[:assigned] = serialize.call(events)
  room.memberships.find_by!(user: bot).destroy!
  events = thread.send(:record_work_assignment_events!, from_owner: bot, to_owner: nil, actor: nil)
  thread.send(:deliver_work_assignment_webhooks, events)
  results[:nonmember] = serialize.call(events)
  Membership.create!(user: bot, room: room)
  AgentGrant.create!(agent: agent, capability: "read_messages", room: room, granted_by: human).revoke!
  events = thread.send(:record_work_assignment_events!, from_owner: bot, to_owner: nil, actor: human)
  thread.send(:deliver_work_assignment_webhooks, events)
  results[:revoked] = serialize.call(events)
  AgentGrant.delete_all
  AgentEvent.delete_all
  agent.agent_events.create!(event_type: "mention", actor: human, room: room, outcome: "acknowledged", chain_id: "ws11-trigger-chain", hop: 2)
  agent.agent_events.create!(event_type: "work_assigned", actor: bot, room: room, outcome: "delivered", chain_id: "self", hop: 9)
  results[:suppressed] = serialize.call(thread.send(:record_work_assignment_events!, from_owner: nil, to_owner: bot, actor: bot))
  handoff = Struct.new(:payload).new({ "summary" => "Take over", "empty" => "", "flag" => false })
  events = thread.send(:record_handoff_events!, from_owner: bot, receiver_agent: agent, sender: human, handoff: handoff)
  thread.send(:deliver_work_assignment_webhooks, events)
  results[:handoff] = serialize.call(events)
  results[:deleted_snapshot] = Agent::Delivery.work_payload(thread, assigned_by: human.name).as_json
  thread.deleted_by = human
  thread.destroy!
  results[:deleted] = serialize.call([agent.agent_events.where(event_type: "work_unassigned").order(:id).last])
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", results: results }.as_json)
end
