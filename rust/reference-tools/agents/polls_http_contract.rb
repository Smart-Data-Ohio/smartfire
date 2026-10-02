# Rails request/response bytes for shared poll creation and results.
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
question="Choose α & β"
variants=[{}, {multiple:"TRUE",anonymous:"off"}, {multiple:0,anonymous:"yes"}, {closes_at:"2026-03-03T16:00:00Z"}, {closes_at:"2026-03-03"}, {closes_at:"17:00"}, {closes_at:false}, {closes_at:"bad",options:[]}, {closes_at:"2026-03-02T16:00:00Z"}, {options:[" A ","",nil,"B"]}, {options:[true,false,3]}, {options:[["A"],{x:"B"}]}, {options:["same","same"]}, {options:["x"*200,"B"]}, {options:["x"*201,"B"]}, {options:["A"]}, {options:(1..10).map(&:to_s)}, {options:(1..11).map(&:to_s)}, {question:" "}, {question:false}, {question:true}, {question:["A","B"]}, {question:{x:1}}, {options:["\u00a0A\u00a0","B"]}, {options:["\0A\0","B"]}, {question:1}, {question:1.5}]
# Date._parse grammar and request coercions, exercised through both transports.
variants += [
 {closes_at:"March 3, 2026 5pm"}, {closes_at:"3 Mar 2026 17:00 UTC"},
 {closes_at:"2026/03/03 17:00"}, {closes_at:"03/03/2026 17:00"},
 {closes_at:"Tue, 03 Mar 2026 17:00:00 GMT"}, {closes_at:"2026-03-03T17:00:00+05:30"},
 {closes_at:"2026-03-03 17:00 EST"}, {closes_at:"2026-03-03 17:00 PDT"},
 {closes_at:"2026-03-03 17:00 MART"}, {closes_at:"2026-03-03 24:00"},
 {closes_at:"2026-03-03 17:00:60"}, {closes_at:"2026-02-30 17:00"},
 {closes_at:"2026-03-00"}, {closes_at:"2026-13-03"}, {closes_at:"2026-03-32"},
 {closes_at:"March 2027"}, {closes_at:"Dec"}, {closes_at:"junk"}, {closes_at:"Tuesday"},
 {closes_at:"03"}, {closes_at:"17:30"}, {closes_at:"5pm"}, {closes_at:"17:00 UTC+0530"},
 {closes_at:"17:00 -05:00"}, {closes_at:"17:00 bananas"},
 {closes_at:"2026-03-03 17:00:00.123456789"},
 {closes_at:[]}, {closes_at:{}}, {closes_at:3}, {closes_at:true}, {closes_at:["2026-03-03"]},
 {closes_at:"\u00a0"}, {closes_at:"\0"},
 {options:nil}, {options:"A"}, {options:false}, {options:{A:"x",B:"y"}},
 {options:["\u00a0","A","B"]}, {options:["\0","A","B"]},
 {options:["α"*200,"B"]}, {options:["α"*201,"B"]},
 {question:"\u00a0"}, {question:"\0"}, {question:nil},
 {multiple:[],anonymous:{}}, {multiple:"False",anonymous:"Off"}
]
variants.each_with_index do |variant,i|
 args={room_id:room,question:question,options:["A","B"]}.merge(variant)
 add.call("poll_create_#{i}",:post,"/rooms/#{room}/agents/polls",args)
 tool.call("create_poll",args,{},"poll_create_#{i}")
end
[{grant:["read_messages"]},{grant:["post_messages"],revoked:true},{remove_member:true},{message_cap:0}].each_with_index do |setup,i|
 args={room_id:room,question:question,options:["A","B"],closes_at:"bad"}
 add.call("poll_authority_#{i}",:post,"/rooms/#{room}/agents/polls",args,setup)
 tool.call("create_poll",args,setup,"poll_authority_#{i}")
end
[{}, {votes:true}, {anonymous:true,votes:true}, {closed:true,votes:true}, {grant:["read_messages"]}, {remove_member:true}, {grant:["post_messages"],revoked:true}].each_with_index do |setup,i|
 setup=setup.merge(poll:true)
 add.call("poll_show_#{i}",:get,"/rooms/#{room}/agents/polls/1900800001",nil,setup)
 tool.call("get_poll",{room_id:room,poll_id:1900800001},setup,"poll_show_#{i}")
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
    agent.update_columns(daily_message_cap:item[:setup][:message_cap])
    ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1900800000 WHERE name='polls'")
    ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1900810000 WHERE name='poll_options'")
    if item[:setup][:poll]
     poll=Poll.create_for_message!(message:Message.find(base),labels:["Small","Large"],multiple:true,anonymous:!!item[:setup][:anonymous])
     if item[:setup][:votes]
      PollVote.create!(poll:poll,poll_option:poll.poll_options.first,user_id:394959859)
      PollVote.create!(poll:poll,poll_option:poll.poll_options.first,user_id:127326141)
      PollVote.create!(poll:poll,poll_option:poll.poll_options.last,user_id:149087659)
     end
     poll.update_columns(closed_at:Time.current) if item[:setup][:closed]
    end
    Rails.cache=ActiveSupport::Cache::MemoryStore.new
    session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
    body=item[:body]&.to_json
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
 puts JSON.pretty_generate({reference_pin:"d7c7de92",cases:results})
end
