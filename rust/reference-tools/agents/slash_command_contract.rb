require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  room = Room.find(486777696)
  actor = User.find(127326141)
  room.memberships.grant_to([agent.user])
  agent.agent_grants.delete_all
  agent.agent_slash_commands.delete_all
  agent.agent_events.delete_all
  validation = {}
  { blank: { name: "" }, invalid: { name: "Deploy!" }, slash: { name: "/deploy" }, spaces: { name: "two words" }, long: { name: "x" * 33 }, builtin: { name: "poll" }, description: { description: "é" * 141 }, boundary: { description: "é" * 140 }, missing_agent: { agent: nil }, missing_room: { room: nil } }.each do |key, fields|
    command = AgentSlashCommand.new({ agent: agent, room: room, name: "deploy" }.merge(fields))
    command.valid?; validation[key] = command.errors.to_hash
  end
  registrations = {}
  capture = lambda do |key, result|
    code = result.status == :unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status)
    registrations[key] = { status: code, payload: result.payload, error: result.error }
  end
  capture.call(:missing, Agents::SlashCommands.register(agent: agent, room_id: 0, name: "inspect"))
  capture.call(:created, Agents::SlashCommands.register(agent: agent, room_id: room.id, name: " Inspect ", description: " Inspect <>& ", takes_arguments: false))
  capture.call(:updated, Agents::SlashCommands.register(agent: agent, room_id: room.id, name: "INSPECT", description: " Again "))
  capture.call(:default, Agents::SlashCommands.register(agent: agent, room_id: room.id, name: "ping"))
  other = Agent.create!(user: User.find_by!(name: "Jason"), owner: actor, kind: :workspace)
  room.memberships.grant_to([other.user])
  capture.call(:foreign, Agents::SlashCommands.register(agent: other, room_id: room.id, name: "inspect"))
  capture.call(:builtin, Agents::SlashCommands.register(agent: agent, room_id: room.id, name: "poll"))
  capture.call(:unregister_foreign, Agents::SlashCommands.unregister(agent: other, room_id: room.id, name: "inspect"))
  capture.call(:unregister, Agents::SlashCommands.unregister(agent: agent, room_id: room.id, name: " PING "))
  capture.call(:unregister_missing, Agents::SlashCommands.unregister(agent: agent, room_id: room.id, name: "ping"))
  duplicate = AgentSlashCommand.new(agent: other, room: room, name: "inspect")
  duplicate.valid?; validation[:duplicate] = duplicate.errors.to_hash
  invocations = {}
  dispatch = lambda do |key, text, thread = nil|
    invocations[key] = SlashCommands::Dispatcher.dispatch(user: actor, room: room, thread: thread, text: text).to_h
  end
  dispatch.call(:root, "/INSPECT   hello <>&")
  thread = room.channel_threads.create!(id: 900_040_001, creator: actor, name: "Slash")
  dispatch.call(:thread, "/inspect x\n y", thread)
  events = agent.agent_events.where(event_type: "slash_command").order(:id).map { |e| { event_type: e.event_type, room_id: e.room_id, actor_id: e.actor_id, outcome: e.outcome, webhook_status: e.webhook_status, chain_uuid: e.chain_id.match?(/\A[0-9a-f-]{36}\z/), metadata: e.metadata } }
  agent.agent_events.delete_all
  20.times { SlashCommands::Dispatcher.dispatch(user: actor, room: room, text: "/inspect flood") }
  dispatch.call(:rate, "/inspect overflow")
  invocations[:rate_count] = agent.agent_events.where(event_type: "slash_command").count
  grant = agent.agent_grants.create!(capability: "post_messages", room: room, granted_by: actor)
  grant.revoke!
  dispatch.call(:revoked, "/inspect nope")
  capture.call(:revoked_registration, Agents::SlashCommands.register(agent: agent, room_id: room.id, name: "inspect"))
  puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], validation: validation, registrations: registrations, invocations: invocations, events: events }.as_json)
end
