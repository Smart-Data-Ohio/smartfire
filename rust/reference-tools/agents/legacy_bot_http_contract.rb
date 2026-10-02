# Legacy bot HTTP and callback state, with real after-commit delivery callbacks.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
legacy_id=1901100001; receiver_id=1901100002; room_id=486777696; base=1901100100
cases=[]
add=->(name,method=:post,body="Legacy hello",setup={},suffix="") { cases << {name:name,method:method,body:body,setup:setup,suffix:suffix} }
add.call("legacy_create")
add.call("legacy_create_utf8",:post,"Hello 👋 α & β")
add.call("legacy_create_blank",:post,"\u00a0")
add.call("legacy_create_repeat",:post,"Repeat",{repeat:1},"?message[client_message_id]=legacy-repeat")
add.call("legacy_create_reply",:post,"Reply",{reply:true})
add.call("legacy_read",:get,nil)
add.call("legacy_read_reply_denied",:get,nil,{reply:true})
add.call("legacy_nonmember",:post,"Hidden",{remove_sender:true})
add.call("legacy_board",:post,"Board",{board:true})
add.call("legacy_update",:patch,"Edited")
add.call("legacy_update_blank",:patch,"")
add.call("legacy_update_other",:patch,"Edited",{other_creator:true})
add.call("legacy_update_system",:patch,"Edited",{system:true})
add.call("legacy_update_reply",:patch,"Edited",{reply:true})
add.call("legacy_destroy",:delete,nil)
add.call("legacy_destroy_other",:delete,nil,{other_creator:true})
add.call("legacy_destroy_system",:delete,nil,{system:true})
add.call("race_budget_first",:post,"Race",{agent_sender:true,budget_slots:1})
add.call("race_budget_overflow",:post,"Race",{agent_sender:true,budget_slots:1,repeat:1})
add.call("race_replay",:post,"Race",{agent_sender:true,budget_slots:1,repeat:1},"?message[client_message_id]=race-replay")
[{}, {hop:0}, {hop:1}, {hop:2}, {inactive:true}, {remove_receiver:true}, {no_webhook:true}].each_with_index do |setup,i|
 add.call("fanout_agent_legacy_#{i}",:post,"Hey {{receiver}}",setup.merge(agent_sender:true))
end
add.call("fanout_agent_self",:post,"Hey {{sender}}",{agent_sender:true})
add.call("fanout_agent_duplicate",:post,"Hey {{receiver}} and {{receiver}}",{agent_sender:true})
add.call("fanout_legacy_self",:post,"Hey {{sender}}")
add.call("fanout_legacy_agent",:post,"Hey {{bender}}")
add.call("fanout_legacy_agent_read_revoked",:post,"Hey {{bender}}",{revoke_receiver:true})
add.call("fanout_legacy_duplicate",:post,"Hey {{receiver}} and {{receiver}}")
add.call("fanout_legacy_plain_named",:post,"Hey @[Legacy Receiver]")
[false,true].each do |agent_sender|
 add.call("fanout_direct_#{agent_sender}",:post,"Direct hello",{direct:true,agent_sender:agent_sender})
 add.call("fanout_direct_inactive_#{agent_sender}",:post,"Direct hello",{direct:true,inactive:true,agent_sender:agent_sender})
 [[:body,"Text edit"],[:clear,""],[:invalid,"invalid-signature"],[:replace,9],[:same,13]].each do |kind,value|
  add.call("replacement_#{agent_sender}_#{kind}",:patch,value,{agent_sender:agent_sender,attachment_old:13,attachment_kind:kind})
 end
 add.call("replacement_#{agent_sender}_destroy",:delete,nil,{agent_sender:agent_sender,attachment_old:13})
end
travel_to Time.utc(2026,3,2,16) do
 legacy=User.create_bot!(id:legacy_id,name:"Legacy Poster",webhook_url:"https://example.test/legacy-poster")
 receiver=User.create_bot!(id:receiver_id,name:"Legacy Receiver",webhook_url:"https://example.test/legacy-receiver")
 legacy.update_columns(bot_token_digest:User.digest_bot_token("LegacyToken1"))
 receiver.update_columns(bot_token_digest:User.digest_bot_token("ReceiverToken1"))
 bender=User.find(394959859); agent=Agent.find(773018776)
 room=Room.find(room_id)
 [legacy,receiver].each { |bot| room.memberships.grant_to(bot) }
 results=cases.map do |item|
  execution=Rails.application.executor.run!(reset:true)
  begin
  setup=item[:setup]
  Message.where("id >= ?",base).destroy_all
  agent.agent_events.delete_all;agent.agent_grants.delete_all
  agent.update_columns(owner_id:127326141,daily_message_cap:nil,suspended_at:nil)
  Room.where(id:room_id).update_all(type:setup[:direct] ? "Rooms::Direct" : setup[:board] ? "Rooms::Board" : "Rooms::Closed")
  receiver.update_columns(status:setup[:inactive] ? 1 : 0)
  receiver.webhook&.destroy!
  receiver.create_webhook!(url:"https://example.test/legacy-receiver") unless setup[:no_webhook]
  [legacy,receiver,bender].each { |bot| room.memberships.grant_to(bot) }
  room.memberships.where(user_id:receiver.id).delete_all if setup[:remove_receiver]
  room.memberships.where(user_id:legacy.id).delete_all if setup[:remove_sender]
  AgentGrant.create!(agent:agent,capability:"react",granted_by_id:127326141) if setup[:revoke_receiver]
  ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=#{base-1} WHERE name='messages'")
  starter=Message.create!(id:base,room:room,creator:setup[:other_creator] || setup[:agent_sender] ? bender : legacy,markdown_source:"Starter",client_message_id:"legacy-starter")
  starter.attachment.attach(ActiveStorage::Blob.find(setup[:attachment_old])) if setup[:attachment_old]
  starter.update_columns(system_note:true) if setup[:system]
  agent.agent_events.delete_all
  if setup[:budget_slots]
   setup[:budget_cap]=Agents::Budgets.usage(agent)[:messages]+setup[:budget_slots]
   agent.update_columns(daily_message_cap:setup[:budget_cap])
  end
  if setup.key?(:hop)
   agent.agent_events.create!(id:1901100200,event_type:"mention",outcome:"delivered",actor_id:127326141,room:room,message:starter,chain_id:"legacy-http-chain",metadata:{hop:setup[:hop]})
  end
  ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1901100200 WHERE name='agent_events'")
  sender=setup[:agent_sender] ? bender : legacy
  mention=->(bot) { %(<action-text-attachment sgid="#{bot.attachable_sgid}" content-type="application/vnd.campfire.mention"></action-text-attachment>) }
  body=item[:body].is_a?(String) ? item[:body].gsub("{{receiver}}",mention.call(receiver))&.gsub("{{sender}}",mention.call(sender))&.gsub("{{bender}}",mention.call(bender)) : nil
  if setup[:attachment_kind] && setup[:attachment_kind]!=:body
   body={attachment:[:same,:replace].include?(setup[:attachment_kind]) ? ActiveStorage::Blob.find(item[:body]).signed_id : item[:body]}.to_json
  end
  key=setup[:reply] ? sender.reply_token_for(room) : setup[:agent_sender] ? "394959859-BenderToken1" : "#{legacy_id}-LegacyToken1"
  path="/rooms/#{room_id}/#{key}/messages"
  path+="/#{base}" if [:patch,:delete].include?(item[:method])
  path+=item[:suffix]
  Rails.cache=ActiveSupport::Cache::MemoryStore.new
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  before=Message.count
  session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
  (0..(setup[:repeat] || 0)).each do
   session.public_send(item[:method],path,params:body,headers:{"Accept"=>"application/json","Content-Type"=>setup[:attachment_kind] && setup[:attachment_kind]!=:body ? "application/json" : "text/plain"})
  end
  response=session.response
  snapshot_execution=Rails.application.executor.run!(reset:true)
  jobs=ApplicationJob.queue_adapter.enqueued_jobs.filter_map do |job|
   case job[:job].name
   when "Bot::WebhookJob"
    {class:job[:job].name,bot_id:job[:args][0]["_aj_globalid"].split("/").last.to_i,message_id:job[:args][1]["_aj_globalid"].split("/").last.to_i}
   when "Agent::DeliveryJob"
    {class:job[:job].name,event_id:job[:args][0]}
   when "ActiveStorage::AnalyzeJob", "ActiveStorage::PurgeJob"
    {class:job[:job].name,blob_id:job[:args][0]["_aj_globalid"].split("/").last.to_i}
   end
  end
  state=Message.where("id >= ?",base).order(:id).map { |m| {id:m.id,creator_id:m.creator_id,markdown_source:m.markdown_source,plain_text:m.plain_text_body,attachment_blob_id:m.attachment.blob&.id} }
  events=AgentEvent.where("id > 1901100200").order(:id).map { |e| {id:e.id,agent_id:e.agent_id,room_id:e.room_id,message_id:e.message_id,actor_id:e.actor_id,event_type:e.event_type,outcome:e.outcome,hop:e.hop,detail:e.detail} }
  # Reply tokens are generated by each implementation from its test secrets.
  item.merge(body:body,path:path.sub(key,"{key}"),status:response.status,response_body:response.body,response_headers:%w[Content-Type Cache-Control Pragma Retry-After Location X-Total-Count Link].to_h { |k| [k,response.headers[k]] },delta:Message.count-before,state:state,events:events,jobs:jobs)
  ensure
   snapshot_execution&.complete!
   execution.complete!
  end
 end
 puts JSON.pretty_generate(reference_pin:"d7c7de92",cases:results)
end
