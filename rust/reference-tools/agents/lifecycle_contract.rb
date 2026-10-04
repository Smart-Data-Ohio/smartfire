require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent=Agent.find(773018776);actor=User.find(127326141);room=Room.find(486777696)
  Current.user=actor
  agent.update_columns(owner_id:actor.id,suspended_at:nil)
  room.memberships.grant_to([agent.user]);AgentEvent.delete_all;AgentGrant.delete_all;AgentApproval.destroy_all
  agent.set_working_presence!("Thinking")
  grant=agent.agent_grants.create!(capability:"read_messages",room:room,granted_by:actor)
  root=room.messages.create!(id:900090001,creator:agent.user,markdown_source:"Draft",client_message_id:"ws11-quiet-root",streaming:true)
  thread=room.channel_threads.create!(id:900090010,creator:actor,parent_message:root,name:"Quiet")
  reply=thread.messages.create!(id:900090002,room:room,creator:agent.user,markdown_source:"Reply",client_message_id:"ws11-quiet-reply",streaming:true)
  thread.update_columns(locked_at:Time.current)
  pending=agent.agent_approvals.create!(id:900090100,action:"deploy",summary:"Pending",expires_at:1.hour.from_now)
  due=agent.agent_approvals.create!(id:900090101,action:"deploy",summary:"Due",expires_at:1.hour.from_now);due.update_columns(expires_at:1.second.ago)
  approved=agent.agent_approvals.create!(id:900090102,action:"deploy",summary:"Approved",expires_at:1.hour.from_now);approved.update_columns(status:"approved")
  snapshot=->(count) do
    agent.reload
    {cancelled:count,suspended:agent.suspended?,working_presence:agent.working_presence,working_presence_expires_at:agent.working_presence_expires_at,
     grants_revoked:agent.agent_grants.order(:id).map{|g|g.revoked_at.present?},
     messages:[root,reply].map{|m|m.reload;{id:m.id,streaming:m.streaming,updated_at:m.updated_at,streaming_updated_at:m.streaming_updated_at}},
     thread_count:thread.reload.messages_count,
     approvals:[pending,due,approved].map{|a|a.reload;{id:a.id,status:a.status}},
     event_count:agent.agent_events.count,
     audits:AuditLog.where(target_type:"Agent",target_id:agent.id,action:["agent.suspend","agent.kill_switch"]).order(:id).map{|a|{action:a.action,actor_id:a.actor_id,actor_label:a.actor_label,target_label:a.target_label,details:a.details}}}
  end
  results={first:snapshot.call(agent.kill_switch!),second:snapshot.call(agent.kill_switch!)}
  results[:quiet_again]=[root.finalize_stream_quietly!,reply.finalize_stream_quietly!]
  puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],results:results}.as_json)
end
