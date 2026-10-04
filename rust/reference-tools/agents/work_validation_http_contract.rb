# Validation and permission errors before the explicitly missing WS12 mutation seam.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
secret = "ws11api-fixture-credential"
room = 486777696
base = 1900700001
thread = 1900700020
cases = []
add = ->(name,method,path,body=nil,setup={}) { cases << {name:name,method:method,path:path,body:body,setup:setup} }
tool = ->(name,args={},setup={},label=name) { add.call("mcp_#{label}",:post,"/agents/mcp",{jsonrpc:"2.0",id:13,method:"tools/call",params:{name:name,arguments:args}},setup) }
grants={grant:["read_messages","post_messages","manage_threads"]}
updates=[{}, {note:"Only note"}, {work_status:nil}, {work_status:""}, {work_status:false}, {work_status:true}, {work_status:[]}, {work_status:{x:1}}, {work_status:"bad",note:"x"*501}, {work_status:"done",note:"x"*501}, {work_status:"done",note:["x"*501]}, {run_url:"http://example.test"}, {run_url:"HTTPS://example.test"}, {run_url:"https://"+"x"*500}, {run_url:true}, {run_url:["https://example.test"]}, {tags:["one","two","three","four","five","six"]}, {tags:["UPPER _"]}, {tags:["x"*31]}, {tags:[{x:"bad"}]}, {tags:[["bad"]]}, {tags:["ok","not_ok","not_ok"]}, {run_url:"bad",tags:["bad_tag"]}]
updates.each_with_index do |args,i|
 add.call("validation_work_#{i}",:patch,"/agents/work/#{thread}",args,grants)
 tool.call("update_work",args.merge(work_id:thread),grants,"validation_work_#{i}")
 tool.call("update_board_post",args.merge(post_id:thread),grants,"validation_board_update_#{i}")
end
add.call("validation_work_nested",:patch,"/agents/work/#{thread}",{work:{run_url:"bad"}},grants)
add.call("validation_work_top_wins",:patch,"/agents/work/#{thread}",{work_status:"bad",work:{work_status:"done"}},grants)
[{}, {markdown:"x"*20001}, {markdown:["x"*20001]}].each_with_index do |args,i|
 add.call("validation_result_#{i}",:put,"/agents/work/#{thread}/result",args,grants)
 tool.call("set_result",args.merge(post_id:thread),grants,"validation_result_#{i}")
end
[0,773018776,{},[],true,"bad"].each_with_index do |receiver,i|
 args={receiver_agent_id:receiver,summary:""}
 add.call("validation_handoff_receiver_#{i}",:post,"/agents/work/#{thread}/handoff",args,grants)
 tool.call("handoff_work",args.merge(work_id:thread,summary:"Ready"),grants,"validation_handoff_receiver_#{i}")
end
[{remove_member:true},{other_owner:true},{grant:["post_messages","manage_threads"]},{grant:["read_messages"]},{grant:["read_messages","manage_threads"],revoked:true}].each_with_index do |setup,i|
 args={work_status:"bad",note:"x"*501,run_url:"bad",tags:["bad_tag"]}
 add.call("validation_work_authority_#{i}",:patch,"/agents/work/#{thread}",args,setup)
 tool.call("update_work",args.merge(work_id:thread),setup,"validation_work_authority_#{i}")
end
travel_to Time.utc(2026,3,2,16) do
 results=cases.map do |item|
  result=nil
  execution=Rails.application.executor.run!(reset:true)
  begin
   ActiveRecord::Base.transaction do
    agent=Agent.find(773018776)
    agent.agent_events.delete_all; agent.agent_grants.delete_all; agent.agent_credentials.delete_all
    agent.update_columns(status:"idle",owner_id:127326141)
    agent.agent_credentials.create!(name:"HTTP contract",created_by_id:127326141,token_digest:AgentCredential.digest(secret),token_last_four:AgentCredential.digest(secret).first(4))
    Array(item[:setup][:grant]).each { |cap| AgentGrant.create!(agent:agent,capability:cap,room_id:item[:setup][:grant_room],granted_by_id:127326141,revoked_at:item[:setup][:revoked] ? Time.current : nil) }
    ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=#{base-1} WHERE name='messages'")
    Message.create!(room_id:room,creator_id:127326141,markdown_source:"Source α & β",client_message_id:"read-source")
    Message.create!(room_id:room,creator_id:394959859,markdown_source:"Agent",client_message_id:"read-agent")
    ActiveRecord::Base.connection.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,locked_at,last_activity_at,created_at,updated_at) VALUES(#{thread},'Owned α & β',#{room},394959859,#{item[:setup][:other_owner] ? 127326141 : 394959859},'in_progress',#{item[:setup][:locked] ? "'2026-03-02 16:00:00'" : 'NULL'},'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
    ThreadTag.create!(channel_thread_id:thread,name:"api")
    Message.create!(room_id:room,creator_id:394959859,thread_id:thread,markdown_source:"Thread",client_message_id:"read-thread")
    Room.where(id:room).update_all(type:"Rooms::Board") if item[:setup][:board]
    Membership.where(room_id:room,user_id:394959859).delete_all if item[:setup][:remove_member]
    MessagePin.pin!(message:Message.find(base),pinner:agent.user) if item[:setup][:pin]
    if item[:setup][:cap]
     (0...50).each do |i|
      message=i==0 ? Message.find(base) : Message.create!(room_id:room,creator_id:127326141,markdown_source:"Cap #{i}",client_message_id:"cap-#{i}")
      MessagePin.create!(message:message,room_id:room,pinner_id:agent.user_id)
     end
    end
    Rails.cache=ActiveSupport::Cache::MemoryStore.new
    session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
    body=item[:body]&.to_json
    headers={"Accept"=>"application/json","Content-Type"=>"application/json","Authorization"=>["Bearer",secret].join(" ")}
    (item[:setup][:repeat] || 0).times {session.public_send(item[:method],item[:path],params:body,headers:headers)}
    session.public_send(item[:method],item[:path],params:body,headers:{"Accept"=>"application/json","Content-Type"=>"application/json","Authorization"=>["Bearer",secret].join(" ")})
    response=session.response
    result=item.merge(body:body,status:response.status,response_body:response.body,response_headers:%w[Content-Type Cache-Control Pragma Retry-After Location].to_h { |key| [key,response.headers[key]] })
    raise ActiveRecord::Rollback
   end
  ensure
   execution.complete!
  end
  result
 end
 puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:results})
end
