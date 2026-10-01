require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  cases = {}
  frames = []
  ActionCable.server.define_singleton_method(:broadcast) { |stream, body, *args, **kwargs| frames << {stream: stream, body: body} }
  agent_id = 773018776
  user_id = 394959859
  room_id = 486777696
  owner_id = 127326141
  reset = lambda do
    Current.reset
    Agent.find(agent_id).update_columns(owner_id: owner_id, suspended_at: nil, working_presence: nil, working_presence_expires_at: nil)
    User.find(user_id).update_columns(status: 0)
    Room.find(room_id).memberships.grant_to([User.find(user_id)])
    AgentGrant.delete_all
    AgentApproval.destroy_all
    AgentEvent.delete_all
    AuditLog.delete_all
    Message.where(client_message_id: %w[ws11-security-kill ws11-security-suspend ws11-security-rollback]).destroy_all
    Membership.where(room_id: room_id).update_all(unread_at: nil)
    ApplicationJob.queue_adapter.enqueued_jobs.clear
    frames.clear
    Agent.find(agent_id)
  end
  grant = ->(room = nil, capability = "post_messages") do
    AgentGrant.create!(agent_id: agent_id, room: room, capability: capability, granted_by_id: owner_id)
  end
  %w[membership workspace rollback room suspend deactivate ban destroy].each do |kind|
    reset.call
    room = Room.find(room_id)
    scoped = grant.call(room)
    extra = kind == "membership" ? grant.call(Room.find(Zlib.crc32("designers") % (2**30 - 1))) : grant.call(nil, "react")
    if kind == "workspace"
      scoped.destroy!
      extra.destroy!
      extra = grant.call
    end
    snapshot = -> do
      agent = Agent.find_by(id: agent_id)
      {scoped_revoked: AgentGrant.find_by(id: scoped.id)&.revoked?, extra_revoked: extra.reload.revoked?,
       member: Membership.exists?(user_id: user_id, room_id: room_id),
       can_post: agent&.can?(:post_messages, room), can_react: agent&.can?(:react, room)}
    end
    ActiveRecord::Base.transaction do
      case kind
      when "membership", "workspace", "rollback" then Membership.find_by!(user_id: user_id, room_id: room_id).destroy!
      when "room" then room.destroy!
      when "suspend" then Agent.find(agent_id).suspend!
      when "deactivate" then User.find(user_id).deactivate
      when "ban" then User.find(user_id).ban
      when "destroy" then User.find(user_id).destroy!
      end
      cases["revoke_#{kind}"] = snapshot.call
      raise ActiveRecord::Rollback
    end
    cases["revoke_rollback_after"] = snapshot.call if kind == "rollback"
  end
  agent = reset.call
  pending = AgentApproval.create!(agent: agent, action: "deploy", summary: "Ship it")
  expired = AgentApproval.create!(agent: agent, action: "old", summary: "Stale")
  expired.update_columns(expires_at: 1.hour.ago)
  approved = AgentApproval.create!(agent: agent, action: "done", summary: "Over")
  approved.update_columns(status: "approved", decided_by_id: owner_id, decided_at: Time.current)
  grant.call
  agent.set_working_presence!("Thinking…")
  cancelled = agent.kill_switch!
  cases[:kill] = {cancelled: cancelled, suspended: agent.reload.suspended?, statuses: [pending, expired, approved].map { |a| a.reload.status },
    presence: agent.working_presence, active_grants: agent.agent_grants.active.count,
    audits: AuditLog.where(action: %w[agent.suspend agent.kill_switch]).order(:id).map { |a| {action: a.action, details: a.details} }}
  cases[:kill_inbox] = {handled: ActivityItem.where(source: pending, user_id: owner_id).sole.handled?}
  again = agent.kill_switch!
  cases[:kill_repeat] = {cancelled: again, suspended: agent.reload.suspended?, suspend_audits: AuditLog.where(action: "agent.suspend").count,
    kill_audits: AuditLog.where(action: "agent.kill_switch").count}
  agent = reset.call
  approval = AgentApproval.create!(agent: agent, action: "github.comment", summary: "Comment", room_id: room_id)
  approval.update_columns(status: "approved", decided_by_id: owner_id, decided_at: Time.current)
  agent.kill_switch!
  Github::PerformAgentActionJob.perform_now(approval.id)
  event = agent.agent_events.where(event_type: "github_action_completed").sole
  cases[:kill_external] = {status: event.metadata["status"], message: event.metadata["message"], events: agent.agent_events.where(event_type: "github_action_completed").count}
  %w[kill suspend rollback].each do |kind|
    agent = reset.call
    room = Room.find(room_id)
    message = room.root_messages.create!(creator_id: user_id, streaming: true,
      markdown_source: kind == "rollback" ? "Still thinking" : "Hey @[David] hovercraft", client_message_id: "ws11-security-#{kind}")
    frames.clear
    ApplicationJob.queue_adapter.enqueued_jobs.clear
    if kind == "rollback"
      Agent.transaction do
        agent.suspend!
        raise ActiveRecord::Rollback
      end
    else
      kind == "kill" ? agent.kill_switch! : agent.suspend!
    end
    stream = [room.to_gid_param, :messages].join(":")
    unread = UnreadRoomsChannel.stream_name_for(owner_id)
    cases["quiet_#{kind}"] = {streaming: message.reload.streaming?, suspended: agent.reload.suspended?,
      activity: ActivityItem.where(source: message).count, events: agent.agent_events.where(message_id: message.id).count,
      search: room.messages.search("hovercraft").map(&:id), unread: Membership.find_by!(room: room, user_id: owner_id).unread_at,
      jobs: ApplicationJob.queue_adapter.enqueued_jobs.size,
      message_broadcasts: frames.count { |f| f[:stream] == stream }, unread_broadcasts: frames.count { |f| f[:stream] == unread }}
  end
  puts JSON.pretty_generate(reference_pin: "d7c7de92", cases: cases)
end
