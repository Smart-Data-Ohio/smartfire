# Rails MCP reaction coercions, canonical shortcode replay, and policy bytes.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
$wire_reactions = []
ActionCable.server.singleton_class.prepend(Module.new do
 def broadcast(name, message, **options)
  $wire_reactions << message if message.is_a?(String) && message.include?('target="boosts_message_read-source"')
  super
 end
end)
Rails.logger = ActiveSupport::Logger.new($stderr)
secret = "ws11api-fixture-credential"
room = 486777696
base = 1900700001
thread = 1900700020
cases = []
add = ->(name,method,path,body=nil,setup={}) { cases << {name:name,method:method,path:path,body:body,setup:setup} }
tool = ->(name,args={},setup={},label=name) { add.call("mcp_#{label}",:post,"/agents/mcp",{jsonrpc:"2.0",id:13,method:"tools/call",params:{name:name,arguments:args}},setup) }
["👍", ":+1:", " :thumbsup: ", ":gpt:", ":claude:", ":unknown:", "Free α & β", "\u00a0", "\0", "\0👍\0", true, 3, 1.5, ["A", 2], {x: "B"}].each_with_index do |content,i|
 tool.call("react",{message_id:base,content:content},{},"react_content_#{i}")
end
tool.call("react",{message_id:base,content: ":+1:"},{repeat:1},"react_replay")
tool.call("react",{message_id:base,content:":gpt:"},{prior: ":openai:"},"react_alias_replay")
tool.call("react",{message_id:base,content:"α"},{prior:"β"},"react_different")
[{}, {content:"👍"}, {message_id:base}, {message_id:base,content:nil}, {message_id:base,content:false}, {message_id:base,content:[]}, {message_id:base,content:{}}].each_with_index { |args,i| tool.call("react",args,{},"react_required_#{i}") }
[{grant:["read_messages"]},{grant:["react"],revoked:true},{grant:["react"],grant_room:201306877},{remove_member:true}].each_with_index do |setup,i|
 tool.call("react",{message_id:base,content:"👍"},setup,"react_authority_#{i}")
end
[0,"no",true,[],{},[base],"#{base}garbage",1900700003].each_with_index { |id,i| tool.call("react",{message_id:id,content:"👍"},{},"react_id_#{i}") }
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
    ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1900900000 WHERE name='boosts'")
    Message.find(base).boosts.create!(booster:agent.user,content:item[:setup][:prior]) if item[:setup][:prior]
    Rails.cache=ActiveSupport::Cache::MemoryStore.new
    session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
    body=item[:body]&.to_json
    headers={"Accept"=>"application/json","Content-Type"=>"application/json","Authorization"=>["Bearer",secret].join(" ")}
    (item[:setup][:repeat] || 0).times {session.public_send(item[:method],item[:path],params:body,headers:headers)}
    $wire_reactions.clear
    session.public_send(item[:method],item[:path],params:body,headers:{"Accept"=>"application/json","Content-Type"=>"application/json","Authorization"=>["Bearer",secret].join(" ")})
    response=session.response
    result=item.merge(body:body,status:response.status,response_body:response.body,broadcasts:$wire_reactions.dup,response_headers:%w[Content-Type Cache-Control Pragma Retry-After Location].to_h { |key| [key,response.headers[key]] })
    raise ActiveRecord::Rollback
   end
  ensure
   execution.complete!
  end
  result
 end
 puts JSON.pretty_generate({reference_pin:"d7c7de92",cases:results})
end
