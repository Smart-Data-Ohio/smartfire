require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776); actor = User.find(127326141)
  room = Rooms::Open.create_for({id: 900080020, name: "Context room", creator: actor}, users: [actor,agent.user])
  room.memberships.grant_to([agent.user]); agent.agent_grants.delete_all
  root = room.messages.create!(id: 900080001, creator: actor, markdown_source: "Root")
  thread = room.channel_threads.create!(id: 900080010, creator: actor, parent_message: root, name: "Context")
  messages = [root]
  3.times { |n| messages << thread.messages.create!(id: 900080002+n, room: room, creator: n == 1 ? agent.user : actor, markdown_source: "Reply #{n}", streaming: n == 2) }
  presenter = Object.new
  presenter.define_singleton_method(:caching_thread_payloads) { |&block| block.call }
  presenter.define_singleton_method(:message_payload) { |m| {id: m.id, creator: {id: m.creator_id, name: m.creator.name, preserved: "yes"}, streaming: m.streaming} }
  results = {}
  capture = ->(key, **attrs) do
    result = Agents::ContextBuilder.build(agent: agent.reload, presenter: presenter, **attrs)
    results[key] = {status: (result.status == :unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status)),payload: result.payload,error: result.error}
  end
  capture.call(:required)
  capture.call(:missing_message, message_id: 0)
  capture.call(:missing_thread, thread_id: 0)
  capture.call(:root, message_id: root.id, limit: "-2")
  capture.call(:thread, thread_id: thread.id, limit: "2tail")
  capture.call(:trigger, message_id: messages[2].id, limit: "bad")
  capture.call(:mismatch, message_id: root.id, thread_id: thread.id)
  AgentGrant.create!(agent: agent, granted_by: actor, capability: "read_messages", room: room).revoke!
  capture.call(:revoked_mismatch, message_id: root.id, thread_id: thread.id)
  room.memberships.find_by!(user: agent.user).destroy!
  capture.call(:nonmember_mismatch, message_id: root.id, thread_id: thread.id)
  puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], results: results }.as_json)
end
