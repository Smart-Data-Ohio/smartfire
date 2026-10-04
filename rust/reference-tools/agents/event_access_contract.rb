require "active_support/testing/time_helpers"
require "zlib"
extend ActiveSupport::Testing::TimeHelpers
fixture_id = ->(label) { Zlib.crc32(label.to_s) % (2**30 - 1) }
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(fixture_id.call(:bender_agent))
  actor = User.find(fixture_id.call(:david))
  water = Room.find(fixture_id.call(:watercooler))
  designers = Room.find(fixture_id.call(:designers))
  [water, designers].each { |room| room.memberships.grant_to([agent.user]) }
  agent.agent_grants.delete_all
  agent.agent_events.delete_all
  rows = {}
  add = lambda do |label, type, attributes = {}|
    rows[label] = agent.agent_events.create!({ event_type: type, outcome: "pending", metadata: { label: label } }.merge(attributes))
  end
  add.call(:blocked_message, "mention", room: designers, message_id: fixture_id.call(:first))
  add.call(:granted_message, "reply", room: water, message_id: fixture_id.call(:fourth))
  add.call(:missing_message, "mention", room: water, message_id: -1)
  add.call(:missing_event_room, "mention", message_id: fixture_id.call(:fourth))
  add.call(:approval, "approval_decided", room: designers)
  add.call(:github, "github_action_completed")
  add.call(:fizzy, "fizzy_action_completed")
  add.call(:granted_work, "work_assigned", room: water)
  add.call(:blocked_work, "work_unassigned", room: designers)
  add.call(:slash, "slash_command", room: water)
  add.call(:posted, "posted", room: water, outcome: "delivered")
  add.call(:suppressed, "mention", room: water, message_id: fixture_id.call(:fourth), outcome: "suppressed")
  add.call(:null_outcome, "mention", room: water, message_id: fixture_id.call(:fourth), outcome: nil)
  other = Agent.create!(user: actor, owner: actor, kind: "personal")
  rows[:other_agent] = other.agent_events.create!(event_type: "github_action_completed", outcome: "pending", metadata: { label: "other_agent" })
  read = lambda do |limit = 100, since = 0|
    agent.agent_events.readable_by(agent).where("agent_events.id > ?", since)
      .where(outcome: %w[pending delivered acknowledged]).ordered.limit(limit).map { |event| event.metadata["label"] }
  end
  grant = agent.agent_grants.create!(capability: "read_messages", room: water, granted_by: actor)
  pages = { scoped: read.call, first_page: read.call(1), second_page: read.call(1, rows[:granted_message].id) }
  ack = lambda do |label|
    event = rows.fetch(label)
    result = Agents::EventPolling.ack(agent: agent, id: event.id)
    event.reload
    { status: result.status, error: result.error, outcome: event.outcome, webhook_status: event.webhook_status, webhook_attempts: event.webhook_attempts }
  end
  rows[:granted_message].update!(webhook_status: "pending", webhook_attempts: 2)
  acks = {}
  %i[granted_message granted_message blocked_message missing_message missing_event_room blocked_work suppressed posted null_outcome other_agent].each_with_index do |label, i|
    acks["#{label}_#{i}"] = ack.call(label)
  end
  grant.revoke!
  pages[:revoked] = read.call
  acks[:revoked_approval] = ack.call(:approval)
  acks[:revoked_work] = ack.call(:granted_work)
  workspace = agent.agent_grants.create!(capability: "read_messages", granted_by: actor)
  pages[:workspace] = read.call
  # Two matching grants must not duplicate the water rows.
  agent.agent_grants.create!(capability: "read_messages", room: water, granted_by: actor)
  pages[:overlapping] = read.call
  Membership.find_by!(user: agent.user, room: water).destroy!
  pages[:removed_membership] = read.call
  acks[:removed_message] = ack.call(:granted_message)
  acks[:removed_work] = ack.call(:granted_work)
  agent.update!(suspended_at: Time.current)
  acks[:suspended_approval] = ack.call(:approval)
  pages[:suspended] = read.call
  agent.update_columns(suspended_at: nil)
  agent.agent_grants.delete_all
  pages[:legacy] = read.call
  caps = {
    legacy_read: Agent.find(agent.id).has_capability_anywhere?(:read_messages),
    legacy_dm: Agent.find(agent.id).has_capability_anywhere?(:dm_anyone),
    unknown: Agent.find(agent.id).has_capability_anywhere?(:launch_missiles),
    legacy_workspace: Agent.find(agent.id).can?(:read_messages)
  }
  agent.user.update_column(:status, :banned)
  caps[:inactive_user] = Agent.find(agent.id).has_capability_anywhere?(:read_messages)
  agent.user.update_column(:status, :active)
  agent.update_columns(suspended_at: Time.current)
  caps[:suspended] = Agent.find(agent.id).has_capability_anywhere?(:read_messages)
  agent.update_columns(suspended_at: nil)
  water.update_columns(deleted_at: Time.current)
  caps[:deleted_room] = Agent.find(agent.id).can?(:read_messages, water.reload)
  # Return the SQL selection as well as ack results; this is deliberately
  # before EventPolling's presenter-dependent payload/cursor assembly.
  puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], pages: pages, acks: acks, capabilities: caps)
end
