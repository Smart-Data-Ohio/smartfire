# Agents::EventPolling itself, with its public presenter collaborator injected.
# Message HTML belongs to WS8bm; this contract covers polling and cursor assembly.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  room = Room.find(486777696)
  actor = User.find(127326141)
  agent.agent_events.delete_all
  agent.agent_grants.delete_all
  room.memberships.grant_to([agent.user])
  calls = []
  presenter = Object.new
  presenter.define_singleton_method(:message_payload) { |message| calls << message.id; { presented_message_id: message.id } }
  add = lambda do |id, type, **attributes|
    agent.agent_events.create!(id: id, event_type: type, outcome: "delivered", **attributes)
  end
  approval = agent.agent_approvals.create!(id: 900_010_001, action: "release", summary: "Ship <>&", decision_note: "", expires_at: Time.current + 1.hour)
  other = Agent.create!(id: 900_010_002, user: actor, owner: actor)
  foreign = other.agent_approvals.create!(id: 900_010_002, action: "release", summary: "Other")
  thread = room.channel_threads.create!(id: 900_020_001, creator: actor, name: "Inspect", work_status: "planned")
  thread.update_columns(work_owner_id: agent.user_id)
  add.call(900_000_001, "reply", room: room, actor: actor, message_id: 136976342, metadata: { hop: 2 })
  add.call(900_000_002, "github_action_completed", metadata: { action: "github.comment", status: false, url: nil, message: "" })
  add.call(900_000_003, "fizzy_action_completed", room: room, actor: actor, metadata: [])
  add.call(900_000_004, "approval_decided", metadata: { approval_id: approval.id, decided_by: "Fallback", note: "Fallback" })
  add.call(900_000_005, "approval_decided", agent_approval_id: approval.id, metadata: {})
  add.call(900_000_006, "approval_decided", metadata: { approval_id: foreign.id })
  add.call(900_000_007, "work_assigned", room: room, actor: actor, metadata: { thread_id: thread.id, assigned_by: actor.name })
  add.call(900_000_008, "work_handed_off", room: room, metadata: { thread_id: thread.id, handoff: { id: 9, summary: "<>&", links: [], open_questions: [], sender_name: "Sender", receiver_agent_id: agent.id, secret: "omit" } })
  add.call(900_000_009, "work_unassigned", room: room, metadata: { thread_id: 0, work_snapshot: { id: 7, title: "Deleted" } })
  add.call(900_000_010, "work_assigned", room: room, metadata: { thread_id: 0, work_snapshot: { id: 7 } })
  add.call(900_000_011, "work_unassigned", room: room, metadata: { thread_id: 0, work_snapshot: [] })
  add.call(900_000_012, "slash_command", room: room, actor: actor, metadata: { thread_id: thread.id, command: "inspect", arguments: "" })
  add.call(900_000_013, "slash_command", room: room, metadata: [])
  add.call(900_000_014, "mention", room_id: nil, message_id: 136976342)
  add.call(900_000_015, "approval_decided", metadata: { approval_id: 0 })
  pages = {}
  poll = lambda do |name, since: 0, limit: 100|
    calls.clear
    pages[name] = Agents::EventPolling.poll(agent: agent.reload, since: since, limit: limit, presenter: presenter).merge(presenter_calls: calls.dup)
  end
  poll.call(:all)
  poll.call(:first, since: "0tail", limit: "1junk")
  poll.call(:dropped, since: 900_000_013)
  poll.call(:empty, since: 900_000_015)
  agent.update_columns(suspended_at: Time.current)
  poll.call(:suspended)
  agent.update_columns(suspended_at: nil)
  room.update_columns(deleted_at: Time.current)
  poll.call(:soft_deleted_room)
  room.update_columns(deleted_at: nil)
  AgentGrant.create!(agent: agent, granted_by: actor, capability: "read_messages", room: room, revoked_at: Time.current)
  poll.call(:revoked)
  blocked = Room.find_by!(name: "Designers")
  blocked.memberships.grant_to([agent.user])
  AgentGrant.create!(agent: agent, granted_by: actor, capability: "read_messages", room: room)
  add.call(900_000_016, "reply", room: blocked, message_id: 136976342)
  poll.call(:event_room_denied)
  puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], pages: pages }.as_json)
end
