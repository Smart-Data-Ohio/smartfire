# Real committed REST/MCP writes. Reset the private seed between requests; no
# outer transaction suppresses Rails' after_commit callbacks or webhook jobs.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
ApplicationJob.queue_adapter = :test
secret = "ws11api-fixture-credential"
room = 486777696
thread = 1900700020
receiver_user = 1901100001
receiver_id = 1901100002
grants = %w[read_messages post_messages manage_threads]
cases = []
add = ->(surface, name, input={}, setup={}) do
  body = input
  method, path = case surface
  when "rest_create" then [:post,"/rooms/#{room}/agents/posts"]
  when "rest_update" then [:patch,"/agents/work/#{thread}"]
  when "rest_result" then [:put,"/agents/work/#{thread}/result"]
  when "rest_handoff" then [:post,"/agents/work/#{thread}/handoff"]
  else
    tool = {"mcp_create"=>"create_board_post", "mcp_update"=>"update_work", "mcp_board_update"=>"update_board_post", "mcp_result"=>"set_result", "mcp_handoff"=>"handoff_work"}.fetch(surface)
    args = input.dup
    args[surface=="mcp_create" ? :room_id : %w[mcp_board_update mcp_result].include?(surface) ? :post_id : :work_id] ||= surface=="mcp_create" ? room : thread
    body = {jsonrpc:"2.0",id:13,method:"tools/call",params:{name:tool,arguments:args}}
    [:post,"/agents/mcp"]
  end
  cases << {name:"#{surface}_#{name}",surface:surface,method:method,path:path,body:body,setup:{grant:grants,board:true,work_write:true}.merge(setup)}
end
surfaces = %w[rest_create rest_update rest_result rest_handoff mcp_create mcp_update mcp_board_update mcp_result mcp_handoff]
valid = ->(surface) do
  if surface.end_with?("create")
    {title:"Ship α & β",body:"The brief.",tags:[" API ","launch","api"],work_status:"planned",run_url:"https://example.test/run"}
  elsif surface.end_with?("result")
    {markdown:"## Shipped α & β"}
  elsif surface.end_with?("handoff")
    {receiver_agent_id:receiver_id,summary:"Halfway α & β",links:"https://example.test/a\r\nhttps://example.test/a\nHTTP://example.test/b",open_questions:[" Why? ","","Why?","Which?\nNext?"]}
  else
    {work_status:"blocked",note:"Waiting α & β",tags:" API, launch,api ",run_url:"https://example.test/run"}
  end
end
surfaces.each do |surface|
  add.call(surface,"success",valid.call(surface))
  [{grant:["read_messages"]},{grant:["post_messages","manage_threads"]},{grant:grants,grant_room:201306877},{grant:grants,revoked:true},{remove_member:true},{other_owner:true},{inactive:true},{credential_revoked:true},{credential_expired:true},{untracked:true}].each_with_index do |setup,i|
    add.call(surface,"permission_#{i}",surface.end_with?("create") ? {} : surface.end_with?("handoff") ? valid.call(surface).merge(summary:"x"*2001) : {},setup)
  end
  [5,50].each { |size| add.call(surface,"queries_#{size}",valid.call(surface),{size:size}) }
end
%w[rest_create mcp_create].each do |surface|
  [ {},{title:""},{title:"x"*101},{title:false,body:false},{title:["Ship"],body:["Brief"]},{title:{x:1}},{title:"Ship",work_status:false,run_url:false},{title:"Ship",work_status:true},{title:"Ship",work_status:"bad"},{title:"Ship",owner_id:127326141},{title:"Ship",owner_id:receiver_user},{title:"Ship",owner_id:"bad"},{title:"Ship",owner_id:0},{title:"Ship",owner_id:false},{title:"Ship",tags:["a","b","c","d","e","f"]},{title:"Ship",tags:"bad_tag"},{title:"Ship",run_url:"http://example.test"} ].each_with_index { |input,i| add.call(surface,"validation_#{i}",input) }
  add.call(surface,"rule_keeps_owner",valid.call(surface),{rule:true})
  add.call(surface,"budget_before_validation",{},{board_cap:1})
  add.call(surface,"not_board",{},{board:false})
end
%w[rest_update mcp_update mcp_board_update].each do |surface|
  [{work_status:"done"},{work_status:"in_progress",note:"No change"},{tags:nil},{run_url:nil},{run_url:false},{tags:[nil,false,12,["x"]]},{work_status:"done",note:"é"*500},{work_status:"done",note:"é"*501},{work_status:"done",tags:"bad_tag",run_url:"http://example.test"}].each_with_index { |input,i| add.call(surface,"validation_#{i}",input) }
  add.call(surface,"rule_keeps_owner",{tags:"launch"},{rule:true})
end
add.call("rest_update","nested",{work:{work_status:"done",note:"Nested",tags:"launch"}})
add.call("rest_update","top_wins",{work_status:"blocked",work:{work_status:"done",note:"Nested"}})
%w[rest_result mcp_result].each do |surface|
  [nil,"",false,true,["Done"],{x:"Done"},"é"*20000,"é"*20001].each_with_index { |markdown,i| add.call(surface,"validation_#{i}",{markdown:markdown}) }
  add.call(surface,"noop",{markdown:"Already"},{result:"Already"})
  add.call(surface,"clear",{markdown:nil},{result:"Already"})
end
%w[rest_handoff mcp_handoff].each do |surface|
  [{summary:" "},{summary:"é"*2001},{summary:"é"*2000},{links:false,open_questions:false},{links:["ftp://example.test"]},{links:(1..11).map{|i|"https://example.test/#{i}"}},{open_questions:["é"*501]},{open_questions:(1..11).map{|i|"Why #{i}?"}},{receiver_agent_id:773018776},{receiver_agent_id:0}].each_with_index { |input,i| add.call(surface,"validation_#{i}",valid.call(surface).merge(input)) }
  %w[inactive outside no_post no_manage no_read].each { |flag| add.call(surface,"receiver_#{flag}",valid.call(surface),{receiver:flag}) }
end
# Preserve arrays accepted by Active Record ID lookups on every MCP write.
%w[mcp_create mcp_update mcp_board_update mcp_result mcp_handoff].each do |surface|
  key=surface=="mcp_create" ? :room_id : %w[mcp_board_update mcp_result].include?(surface) ? :post_id : :work_id
  add.call(surface,"array_id",valid.call(surface).merge(key=>[0,surface=="mcp_create" ? room : thread]))
end

conn=ActiveRecord::Base.connection
# Restore actual SQL values, including sequence numbers. This is fixture isolation,
# not an outcome mask; all production requests run outside the restore transaction.
tables=conn.tables.reject{|name|name.start_with?("message_search") || name=="schema_migrations" || name=="ar_internal_metadata"}
tables << "sqlite_sequence"
snapshot=tables.to_h{|table|[table,conn.select_all("SELECT * FROM #{conn.quote_table_name(table)}").to_a]}
restore=-> do
  conn.execute("PRAGMA foreign_keys=OFF")
  conn.execute("DELETE FROM message_search_index")
  conn.transaction do
    tables.each{|table|conn.execute("DELETE FROM #{conn.quote_table_name(table)}")}
    snapshot.each do |table,rows|
      rows.each do |row|
        conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map{|key|conn.quote_column_name(key)}.join(',')}) VALUES (#{row.values.map{|value|conn.quote(value)}.join(',')})")
      end
    end
  end
  conn.execute("PRAGMA foreign_keys=ON")
end
stamp=->(time){time&.utc&.iso8601(3)}
results=[]
travel_to Time.utc(2026,3,2,16) do
  cases.each do |item|
    restore.call
    execution=Rails.application.executor.run!(reset:true)
    begin
      setup=item[:setup]
      agent=Agent.find(773018776)
      agent.agent_events.delete_all; agent.agent_grants.delete_all; agent.agent_credentials.delete_all
      agent.update_columns(status:"idle",owner_id:127326141,daily_message_cap:nil,daily_board_post_cap:setup[:board_cap],suspended_at:setup[:inactive] ? Time.current : nil)
      credential=agent.agent_credentials.create!(name:"HTTP contract",created_by_id:127326141,token_digest:AgentCredential.digest(secret),token_last_four:AgentCredential.digest(secret).first(4))
      credential.update_columns(revoked_at:Time.current) if setup[:credential_revoked]
      credential.update_columns(expires_at:Time.current) if setup[:credential_expired]
      setup[:grant].each{|cap|AgentGrant.create!(agent:agent,capability:cap,room_id:setup[:grant_room],granted_by_id:127326141,revoked_at:setup[:revoked] ? Time.current : nil)}
      user=User.create_bot!(id:receiver_user,name:"Receiver")
      receiver=Agent.create!(id:receiver_id,user:user,owner_id:127326141,kind: :workspace)
      user.create_webhook!(url:"https://receiver.example.test/hook")
      Room.find(room).memberships.grant_to(user)
      grants.each{|cap|AgentGrant.create!(agent:receiver,room_id:room,capability:cap,granted_by_id:127326141)}
      receiver.update_columns(suspended_at:Time.current) if setup[:receiver]=="inactive"
      Membership.where(room_id:room,user_id:receiver_user).delete_all if setup[:receiver]=="outside"
      {"no_post"=>"post_messages","no_manage"=>"manage_threads","no_read"=>"read_messages"}.each{|flag,cap|receiver.agent_grants.where(capability:cap).delete_all if setup[:receiver]==flag}
      conn.execute("UPDATE sqlite_sequence SET seq=1900700000 WHERE name='messages'")
      Message.create!(room_id:room,creator_id:127326141,markdown_source:"Source α & β",client_message_id:"read-source")
      Message.create!(room_id:room,creator_id:394959859,markdown_source:"Agent",client_message_id:"read-agent")
      conn.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(#{thread},'Owned α & β',#{room},394959859,#{setup[:other_owner] ? 127326141 : 394959859},'in_progress','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
      ThreadTag.create!(channel_thread_id:thread,name:"api")
      Message.create!(room_id:room,creator_id:394959859,thread_id:thread,markdown_source:"Thread",client_message_id:"read-thread")
      Room.where(id:room).update_all(type:"Rooms::Board") if setup[:board]
      ChannelThread.where(id:thread).update_all(work_status:nil,work_owner_id:nil) if setup[:untracked]
      ChannelThread.where(id:thread).update_all(result_markdown:setup[:result]) if setup[:result]
      Membership.where(room_id:room,user_id:394959859).delete_all if setup[:remove_member]
      BoardTagAssignment.create!(room_id:room,tag:"launch",assignee:user,created_by_id:127326141) if setup[:rule]
      (setup[:size] || 0).times do |i|
        conn.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(#{1901200000+i},'Other #{i}',#{room},394959859,394959859,'planned','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
      end
      %w[channel_threads work_thread_events work_handoffs agent_events audit_logs].each_with_index{|table,i|conn.execute("DELETE FROM sqlite_sequence WHERE name='#{table}'");conn.execute("INSERT INTO sqlite_sequence(name,seq) VALUES('#{table}',#{1901300000+i*1000})")}
      before={history:WorkThreadEvent.maximum(:id)||0,ledger:AgentEvent.maximum(:id)||0,audit:AuditLog.maximum(:id)||0}
      ApplicationJob.queue_adapter.enqueued_jobs.clear
      Rails.cache=ActiveSupport::Cache::MemoryStore.new
      session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
      headers={"Accept"=>"application/json","Content-Type"=>"application/json","User-Agent"=>"ws11api-work-contract","X-Forwarded-For"=>"203.0.113.31","Authorization"=>["Bearer",secret].join(" ")}
      body=item[:body].to_json
      queries=[]
      callback=->(*args){payload=args.last;queries<<payload[:sql] if !payload[:cached] && payload[:sql].match?(/\ASELECT\b/i)}
      ActiveSupport::Notifications.subscribed(callback,"sql.active_record") {session.public_send(item[:method],item[:path],params:body,headers:headers)}
      response=session.response
      written=ChannelThread.where(id:1901300001).first || ChannelThread.find(thread)
      state={thread:{id:written.id,title:written.name,room_id:written.room_id,creator_id:written.creator_id,owner:written.work_owner_id,work_status:written.work_status,tags:written.tag_names,result:written.result_markdown,result_updated_at:stamp.call(written.result_updated_at),result_updated_by:written.result_updated_by_id,run_url:written.run_url,updated_at:stamp.call(written.updated_at),work_status_changed_at:stamp.call(written.work_status_changed_at)},
        messages:written.messages.order(:id).map{|m|{id:m.id,creator_id:m.creator_id,thread_id:m.thread_id,markdown:m.markdown_source,opener:m.board_post_opener}},
        history:WorkThreadEvent.where("id>?",before[:history]).order(:id).map{|e|{id:e.id,thread_id:e.channel_thread_id,kind:e.event_type,actor:e.actor_id,from_owner:e.from_owner_id,to_owner:e.to_owner_id,from_status:e.from_status,to_status:e.to_status,metadata:e.metadata}},
        ledger:AgentEvent.where("id>?",before[:ledger]).order(:id).map{|e|{id:e.id,agent:e.agent_id,room_id:e.room_id,kind:e.event_type,actor:e.actor_id,outcome:e.outcome,hop:e.hop,metadata:e.metadata,webhook_status:e.webhook_status}},
        handoffs:WorkHandoff.where(channel_thread_id:written.id).order(:id).map{|h|{id:h.id,thread_id:h.channel_thread_id,sender_id:h.sender_id,receiver_agent_id:h.receiver_agent_id,summary:h.summary,links:h.links,open_questions:h.open_questions}},
        audit:AuditLog.where("id>?",before[:audit]).order(:id).map{|a|a.attributes.slice("action","actor_id","actor_label","target_type","target_id","target_label","details","ip_address","user_agent")},
        jobs:ApplicationJob.queue_adapter.enqueued_jobs.map do |j|
          args=case j[:job].name
          when "ChannelThread::PushMessageJob" then {thread_id:j[:args][0]["_aj_globalid"].split("/").last.to_i,message_id:j[:args][1]["_aj_globalid"].split("/").last.to_i}
          when "Agent::EventWebhookJob" then {event_id:j[:args][0],attempt:j[:args][1]}
          else raise "unmapped job #{j[:job].name}: #{j[:args]}"
          end
          {class:j[:job].name,args:args}
        end}
      results<<item.merge(body:body,status:response.status,response_body:response.body,response_headers:%w[Content-Type Cache-Control Pragma Retry-After Location].to_h{|key|[key,response.headers[key]]},state:state,selects:queries.size)
      warn "Work write Rails case #{item[:name]}: #{response.status}; #{queries.size} SELECTs"
    rescue Exception => error
      warn error.full_message
      raise
    ensure
      execution.complete!
    end
  end
end
puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],notes:["Each request commits in a private restored seed. Bodies and selected headers are compared as raw bytes. Ledger chains are checked for a shared UUID independently because production UUIDs are random; no response fields are masked."],cases:results)
