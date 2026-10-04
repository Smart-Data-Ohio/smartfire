require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent=Agent.find(773018776); actor=User.find(127326141); target=User.find(149087659)
  Current.user=agent.user
  agent.update_columns(owner_id:actor.id,daily_message_cap:nil)
  agent.user.update_columns(time_zone:"UTC")
  AgentGrant.delete_all;AgentEvent.delete_all
  results={}
  capture=->(key,user_id,attrs={},drive=:absent) do
    result=Agents::DirectMessages.open_and_post(agent:agent.reload,user_id:user_id,attributes:attrs,drive_file_ids:drive)
    payload=result.payload
    if result.ok?
      payload={message:{source:payload[:message].markdown_source,creator_id:payload[:message].creator_id,client_message_id:payload[:message].client_message_id},room:{type:payload[:room].type,members:payload[:room].user_ids.sort}}
    end
    results[key]={status:result.status==:unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status),error:result.error,payload:payload}
  end
  capture.call(:missing,0)
  capture.call(:bot,agent.user_id)
  target.update_columns(status: :deactivated);capture.call(:inactive,target.id);target.update_columns(status: :active)
  capture.call(:unrelated,target.id)
  attrs={markdown_source:"Hello",client_message_id:"ws11-dm-owner"}
  capture.call(:owner,actor.id,attrs)
  agent.update_columns(daily_message_cap:0)
  capture.call(:replay,actor.id,attrs, nil)
  capture.call(:budget,actor.id,{markdown_source:"Other"})
  agent.update_columns(daily_message_cap:nil)
  agent.agent_events.create!(event_type:"mention",actor:target,outcome:"suppressed",created_at:1.year.ago)
  capture.call(:previous_human,target.id,{markdown_source:"Old inbound",client_message_id:"ws11-dm-prior"})
  capture.call(:bad_reply,actor.id,{reply_to_message_id:1,markdown_source:"Reply"})
  capture.call(:invalid_drive,actor.id,{markdown_source:"Drive"},nil)
  capture.call(:blank,actor.id,{markdown_source:""})
  grant=agent.agent_grants.create!(capability:"post_messages",room:Room.find(486777696),granted_by:actor)
  capture.call(:existing_room_scope,actor.id,attrs)
  capture.call(:scope_bot,agent.user_id)
  grant.revoke!;capture.call(:revoked,actor.id,attrs)
  puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],results:results}.as_json)
end
