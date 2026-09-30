require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent=Agent.find(773018776);room=Room.find(486777696);actor=User.find(127326141)
  Current.user=agent.user
  AgentGrant.delete_all;AgentEvent.delete_all;room.memberships.grant_to([agent.user])
  agent.update_columns(daily_message_cap:nil,suspended_at:nil)
  results={}
  capture=->(key,result) do
    payload=result.payload
    payload={source:payload.markdown_source,streaming:payload.streaming,creator_id:payload.creator_id,edited_at:payload.edited_at} if payload.is_a?(Message)
    results[key]={status:result.status==:unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status),payload:payload,error:result.error}
    result
  end
  message=capture.call(:start,Agents::Streaming.start(agent:agent,room:room,attributes:{markdown_source:"",client_message_id:"ws11-stream"})).payload
  capture.call(:append,Agents::Streaming.update(agent:agent,id:message.id,append:"First"))
  capture.call(:replace,Agents::Streaming.update(agent:agent,id:message.id,markdown_source:"Replacement"))
  capture.call(:append_wins,Agents::Streaming.update(agent:agent,id:message.id,append:"!",markdown_source:"ignored"))
  capture.call(:required,Agents::Streaming.update(agent:agent,id:message.id))
  capture.call(:replay,Agents::Streaming.start(agent:agent,room:room,attributes:{markdown_source:"ignored",client_message_id:"ws11-stream"}))
  thread=room.channel_threads.create!(creator:actor,name:"Locked")
  message.update_columns(thread_id:thread.id);thread.update_columns(locked_at:Time.current)
  capture.call(:locked,Agents::Streaming.update(agent:agent,id:message.id,append:"late"))
  capture.call(:locked_finalize,Agents::Streaming.finalize(agent:agent,id:message.id))
  grant=agent.agent_grants.create!(capability:"post_messages",room:room,granted_by:actor);grant.revoke!
  capture.call(:revoked,Agents::Streaming.update(agent:agent,id:message.id,append:"late"))
  room.memberships.find_by!(user:agent.user).destroy!
  capture.call(:nonmember,Agents::Streaming.update(agent:agent,id:message.id,append:"late"))
  AgentGrant.delete_all;room.memberships.grant_to([agent.user])
  final=Agents::Streaming.start(agent:agent.reload,room:room,attributes:{markdown_source:"Final",client_message_id:"ws11-final"}).payload
  agent.set_working_presence!("Finishing")
  capture.call(:finalized,Agents::Streaming.finalize(agent:agent,id:final.id))
  capture.call(:finalized_again,Agents::Streaming.finalize(agent:agent,id:final.id))
  results[:final_side_effects]={posted:agent.agent_events.where(event_type:"posted",message_id:final.id).count,
    indexed:Message.connection.select_value("SELECT COUNT(*) FROM message_search_index WHERE rowid=#{final.id}").to_i,
    presence:agent.reload.working_presence}
  puts JSON.pretty_generate({reference_pin:"d7c7de92",results:results}.as_json)
end
