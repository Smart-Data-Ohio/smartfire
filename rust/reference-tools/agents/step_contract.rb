require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776); room = Room.find(486777696); actor = User.find(127326141)
  room.memberships.grant_to([agent.user]); agent.agent_grants.delete_all
  AgentStep.delete_all
  AgentStep.connection.execute("DELETE FROM sqlite_sequence WHERE name='agent_steps'")
  AgentStep.connection.execute("INSERT INTO sqlite_sequence(name,seq) VALUES ('agent_steps',900050000)")
  message = room.messages.create!(id: 900060001, creator: agent.user, markdown_source: "Working", client_message_id: "ws11-steps-parent")
  foreign = room.messages.create!(id: 900060002, creator: actor, markdown_source: "Mine", client_message_id: "ws11-steps-foreign")
  thread = room.channel_threads.create!(id: 900060003, creator: actor, name: "Work", work_status: "in_progress", work_owner: agent.user)
  chat = room.channel_threads.create!(id: 900060004, creator: actor, name: "Chat")
  base = { agent: agent, message: message, name: "Run tests" }
  validation = {}
  { blank: { name: "" }, no_parent: { message: nil }, both: { channel_thread: thread }, foreign: { message: foreign }, unowned: { message: nil, channel_thread: chat },
    oversized: { name: "é" * 121, input_summary: "é" * 1001, output_summary: "é" * 1001, status: "exploding", duration_ms: -1 },
    boundary: { name: "é" * 120, input_summary: "é" * 1000, duration_ms: 0 }, blank_summary: { input_summary: " " * 2000 }, orphan: { message: nil, message_id: -1 } }.each do |key, fields|
    row = AgentStep.new(base.merge(fields)); row.valid?; validation[key] = row.errors.to_hash
  end
  results = {}
  capture = lambda do |name, result|
    code = result.status == :unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status)
    payload = result.payload.is_a?(AgentStep) ? Agents::Steps.step_payload(result.payload) : result.payload
    results[name] = { status: code, error: result.error, payload: payload }
    result
  end
  fields = { "message_id" => message.id, "name" => "Run tests", "input_summary" => " ", "duration_ms" => 1200 }
  created = capture.call(:created, Agents::Steps.create(agent: agent, fields: fields))
  capture.call(:updated, Agents::Steps.update(agent: agent, id: created.payload.id, fields: { "status" => "done", "output_summary" => "", "duration_ms" => 1500 }))
  capture.call(:no_parent, Agents::Steps.create(agent: agent, fields: { "name" => "Nowhere" }))
  capture.call(:foreign, Agents::Steps.create(agent: agent, fields: fields.merge("message_id" => foreign.id)))
  capture.call(:missing_step, Agents::Steps.update(agent: agent, id: 0, fields: {}))
  grant = agent.agent_grants.create!(capability: "manage_threads", room: room, granted_by: actor)
  capture.call(:thread, Agents::Steps.create(agent: agent, fields: { "thread_id" => thread.id, "name" => "Reproduce", "output_summary" => "Found" }))
  capture.call(:missing_post_grant, Agents::Steps.create(agent: agent, fields: fields))
  capture.call(:revoked_update, Agents::Steps.update(agent: agent, id: created.payload.id, fields: { "name" => "Nope" }))
  grant.revoke!
  capture.call(:revoked_thread, Agents::Steps.create(agent: agent, fields: { "thread_id" => thread.id, "name" => "Nope" }))
  agent.agent_grants.delete_all
  49.times { |n| AgentStep.create!(base.merge(name: "Step #{n}")) }
  overflow = AgentStep.new(base); overflow.valid?; validation[:limit] = overflow.errors.to_hash
  AgentStep.where(message_id: message.id).order(:id).first.destroy!
  replacement = AgentStep.create!(base)
  positions = { replacement: replacement.position, count: AgentStep.where(message_id: message.id).count }
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", validation: validation, results: results, positions: positions }.as_json)
end
