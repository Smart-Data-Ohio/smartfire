# Rails request/response bytes for shared room, history, board and owned-work readers.
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
tool.call("list_rooms")
[%w[read_messages], %w[dm_anyone], %w[react], %w[external_action]].each { |grant| tool.call("list_rooms",{}, {grant:grant},"rooms_#{grant.first}") }
tool.call("list_rooms",{}, {grant:["read_messages"],grant_room:room},"rooms_scoped")
tool.call("list_rooms",{}, {grant:["read_messages"],revoked:true},"rooms_revoked")
tool.call("list_rooms",{}, {grant:["read_messages"],grant_room:201306877},"rooms_nonmember_grant")
[{}, {limit:1}, {limit:2,before:base+1}, {limit:2,after:base}, {limit:1,before:base+1,after:base}, {limit:0}, {limit:"2junk"}, {limit:101}, {limit:false}, {limit:true}, {limit:[]}, {limit:{}}, {before:0}, {after:base+2}].each_with_index { |args,i| tool.call("read_messages",{room_id:room}.merge(args),{},"history_#{i}") }
tool.call("read_messages",{thread_id:thread,limit:2},{},"history_thread")
tool.call("read_messages",{room_id:room},{grant:["react"]},"history_denied")
tool.call("read_messages",{room_id:room},{grant:["read_messages"],revoked:true},"history_revoked")
tool.call("read_messages",{room_id:201306877},{grant:["read_messages"]},"history_nonmember")
tool.call("read_messages",{thread_id:thread},{remove_member:true},"history_removed_member")
tool.call("read_messages",{thread_id:thread},{locked:true},"history_locked")
add.call("work_index",:get,"/agents/work")
add.call("work_show",:get,"/agents/work/#{thread}")
add.call("work_show_other_owner",:get,"/agents/work/#{thread}",nil,{other_owner:true})
add.call("work_show_no_read",:get,"/agents/work/#{thread}",nil,{grant:["post_messages"]})
add.call("work_index_no_read",:get,"/agents/work",nil,{grant:["post_messages"]})
add.call("work_show_removed_member",:get,"/agents/work/#{thread}",nil,{remove_member:true})
tool.call("list_work")
tool.call("list_work",{ignored:"ignored"},{grant:["post_messages"]},"work_no_read")
tool.call("list_work",{},{remove_member:true},"work_removed_member")
[{}, {status:"all"}, {status:"done"}, {status:"planned"}, {owner:"me"}, {owner:"agents"}, {owner:127326141}, {tag:" API "}, {owner:"wrong"}, {status:"invalid"}, {tag:"missing"}].each_with_index do |args,i|
 query=args.map { |k,v| "#{k}=#{CGI.escape(v.to_s)}" }.join("&")
 add.call("posts_#{i}",:get,"/rooms/#{room}/agents/posts#{query.empty? ? '' : '?' + query}",nil,{board:true})
 tool.call("list_board_posts",{room_id:room}.merge(args),{board:true},"posts_#{i}")
end
tool.call("list_board_posts",{room_id:room},{board:true,grant:["post_messages"]},"posts_denied")
# find_by accepts IN lists, including nested lists, and chooses the first matching
# row in the authorized association. Cursor resolution follows limit validation.
{
 room_id:[nil,false,true,0,"",[],{},[room],[0,room],[[room]],{id:room},[201306877,room]],
 thread_id:[nil,false,true,0,"",[],{},[thread],[0,thread],[[thread]],{id:thread}],
 before:[nil,false,true,0,"",[],{},[base+1],[0,base+1],[[base+1]],{id:base+1},[base+2,base+1]],
 after:[nil,false,true,0,"",[],{},[base],[0,base],[[base]],{id:base},[base+2,base]]
}.each do |field,values|
 values.each_with_index do |value,i|
  args=[:before,:after].include?(field) ? {room_id:room,limit:2} : {}
  tool.call("read_messages",args.merge(field=>value),{},"history_shape_#{field}_#{i}")
 end
end
[true,[1],{n:1}].each_with_index do |limit,i|
 tool.call("read_messages",{room_id:room,limit:limit,before:0},{},"history_limit_before_cursor_#{i}")
 tool.call("read_messages",{thread_id:thread,limit:limit,after:0},{grant:["react"]},"history_grant_before_limit_#{i}")
end
[:status,:owner,:tag].each do |field|
 [nil,false,true,0,[],["all"],{k:"all"},"\u00a0all\u00a0","\u0000all\u0000","\tall\r\n"].each_with_index do |value,i|
  args={field=>value}
  add.call("posts_shape_#{field}_#{i}",:get,"/rooms/#{room}/agents/posts?#{Rack::Utils.build_nested_query(args)}",nil,{board:true})
  tool.call("list_board_posts",{room_id:room}.merge(args),{board:true},"posts_shape_#{field}_#{i}")
 end
end
[[room],[0,room],[[room]],{id:room}].each_with_index do |value,i|
 tool.call("list_board_posts",{room_id:value},{board:true},"posts_room_shape_#{i}")
end
# Owner-disallowed and unknown repositories redact only their sensitive details.
# No account or external repository access is fabricated by these vectors.
[false,true,nil].each_with_index do |private,i|
 setup={work_pr:true,pr_private:private}
 add.call("work_pr_show_#{i}",:get,"/agents/work/#{thread}",nil,setup)
 add.call("work_pr_index_#{i}",:get,"/agents/work",nil,setup)
 tool.call("list_work",{},setup,"work_pr_index_#{i}")
 add.call("posts_pr_#{i}",:get,"/rooms/#{room}/agents/posts",nil,setup.merge(board:true))
 tool.call("list_board_posts",{room_id:room},setup.merge(board:true),"posts_pr_#{i}")
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
    if item[:setup][:work_pr]
     pr=Github::PullRequest.create!(id:1900700030,owner:"acme",repo:"secret",number:3,title:"Secret acquisition α & β",state:"open",head_branch:"secret-branch",base_branch:"main",review_decision:"approved",check_status:"passing",private:item[:setup][:pr_private])
     WorkThreadLink.create!(id:1900700031,channel_thread_id:thread,kind:"pull_request",github_pull_request:pr,created_by_id:127326141)
    end
    Message.create!(room_id:room,creator_id:394959859,thread_id:thread,markdown_source:"Thread",client_message_id:"read-thread")
    Room.where(id:room).update_all(type:"Rooms::Board") if item[:setup][:board]
    Membership.where(room_id:room,user_id:394959859).delete_all if item[:setup][:remove_member]
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
