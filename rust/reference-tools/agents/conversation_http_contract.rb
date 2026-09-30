# Actual production Rails wire bodies; every request starts from the same frozen state.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
secret = "ws11api-fixture-credential"
base = 1900600001
thread = 1900600008
room = 486777696
cases = []
add = ->(name, method, path, body=nil, setup={}) { cases << {name:name, method:method, path:path, body:body, setup:setup} }
add.call("context_root", :get, "/agents/context?message_id=#{base}&limit=1")
add.call("context_thread", :get, "/agents/context?thread_id=#{thread}&limit=1")
add.call("context_trigger", :get, "/agents/context?message_id=#{base+2}&thread_id=#{thread}")
add.call("context_no_parent", :get, "/agents/context")
add.call("context_mismatch", :get, "/agents/context?message_id=#{base}&thread_id=#{thread}")
add.call("post_html", :post, "/rooms/#{room}/agents/messages", {message:{body:"<p>Hello &amp; α > β</p>",client_message_id:"wire-post"}})
add.call("post_markdown_reply_drive", :post, "/rooms/#{room}/agents/messages", {message:{body:"ignored",markdown_source:"**Hello** α & β",client_message_id:"wire-post",reply_to_message_id:base,reply_notify_author:false,drive_file_ids:[" 1AbcDefGhIjKlMnOpQrSt ","","1AbcDefGhIjKlMnOpQrSt"]}})
add.call("post_thread", :post, "/rooms/#{room}/agents/messages", {thread_id:thread,message:{markdown_source:"Thread",client_message_id:"wire-post"}})
add.call("post_invalid_drive", :post, "/rooms/#{room}/agents/messages", {message:{body:"Hello",client_message_id:"wire-post",drive_file_ids:{a:1}}})
add.call("post_empty", :post, "/rooms/#{room}/agents/messages", {message:{body:" ",client_message_id:"wire-post"}})
add.call("post_budget", :post, "/rooms/#{room}/agents/messages", {message:{body:"Hello",client_message_id:"wire-post"}}, {cap:0})
add.call("post_replay_over_budget", :post, "/rooms/#{room}/agents/messages", {message:{body:"ignored",client_message_id:"wire-agent",drive_file_ids:nil}}, {cap:0})
add.call("post_thread_locked", :post, "/rooms/#{room}/agents/messages", {thread_id:thread,message:{body:"Hello",client_message_id:"wire-post"}}, {locked:true})
add.call("dm_nested", :post, "/agents/dms", {user_id:127326141,body:"ignored",message:{markdown_source:"DM α & β",client_message_id:"wire-dm"}})
add.call("dm_top_level", :post, "/agents/dms", {user_id:127326141,body:"<p>Hello α &amp; β</p>",client_message_id:"wire-dm"})
add.call("dm_empty", :post, "/agents/dms", {user_id:127326141,message:{body:" ",client_message_id:"wire-dm"}})
add.call("dm_invalid_drive", :post, "/agents/dms", {user_id:127326141,body:"Hello",client_message_id:"wire-dm",drive_file_ids:nil})
[
 ["get_context", {message_id:base+2,thread_id:thread}],
 ["post_message", {room_id:room,body:"<p>Hello α &amp; β</p>",client_message_id:"wire-post"}],
 ["post_message", {room_id:room,markdown_source:"**Hello** α & β",reply_to_message_id:base,reply_notify_author:false,client_message_id:"wire-post",drive_file_ids:["1AbcDefGhIjKlMnOpQrSt"]}],
 ["post_message", {room_id:room,thread_id:thread,markdown_source:"Thread",client_message_id:"wire-post"}],
 ["open_dm", {user_id:127326141,markdown_source:"DM α & β",client_message_id:"wire-dm"}],
 ["post_message", {room_id:room,body:"Hello",client_message_id:"wire-post",drive_file_ids:nil}],
 ["post_message", {room_id:room,body:"Hello",reply_to_message_id:0,client_message_id:"wire-post"}]
].each_with_index do |(tool,args),i|
 add.call("mcp_#{tool}_#{i}", :post, "/agents/mcp", {jsonrpc:"2.0",id:11,method:"tools/call",params:{name:tool,arguments:args}})
end
add.call("stream_start", :post, "/rooms/#{room}/agents/streaming_messages", {message:{markdown_source:"**Start** α & β",client_message_id:"wire-stream",drive_file_ids:["bad"]}})
add.call("stream_replay_over_budget", :post, "/rooms/#{room}/agents/streaming_messages", {message:{markdown_source:"ignored",client_message_id:"wire-agent"}}, {cap:0})
add.call("stream_append", :patch, "/agents/streaming_messages/#{base+1}", {append:" **Next** α & β",markdown_source:"ignored"}, {stream:true})
add.call("stream_replace", :patch, "/agents/streaming_messages/#{base+1}", {markdown_source:"Replacement α & β"}, {stream:true})
add.call("stream_finalize", :post, "/agents/streaming_messages/#{base+1}/finalize", {}, {stream:true})
add.call("stream_finalize_again", :post, "/agents/streaming_messages/#{base+1}/finalize", {})
add.call("stream_not_streaming", :patch, "/agents/streaming_messages/#{base+1}", {append:"Next"})
add.call("stream_no_append", :patch, "/agents/streaming_messages/#{base+1}", {}, {stream:true})
add.call("stream_wrong_author", :post, "/agents/streaming_messages/#{base}/finalize", {})
add.call("stream_thread_start", :post, "/rooms/#{room}/agents/streaming_messages", {thread_id:thread,message:{markdown_source:"Thread stream",client_message_id:"wire-stream"}})
add.call("stream_locked", :patch, "/agents/streaming_messages/#{base+2}", {append:"Next"}, {thread_stream:true,locked:true})
[
 ["start_stream",{room_id:room,markdown_source:"**Start** α & β",client_message_id:"wire-stream"},{}],
 ["append_stream",{message_id:base+1,append:" **Next** α & β"},{stream:true}],
 ["finalize_stream",{message_id:base+1},{stream:true}],
 ["finalize_stream",{message_id:base+1},{}],
 ["append_stream",{message_id:base+1,append:false},{stream:true}],
 ["append_stream",{message_id:base+1,append:{name:"test"}},{stream:true}],
 ["append_stream",{message_id:base+1,append:[]},{stream:true}],
 ["append_stream",{message_id:base+1},{}]
].each_with_index do |(tool,args,setup),i|
 add.call("mcp_stream_#{tool}_#{i}",:post,"/agents/mcp",{jsonrpc:"2.0",id:11,method:"tools/call",params:{name:tool,arguments:args}},setup)
end
travel_to Time.utc(2026,3,2,16) do
 results = cases.map do |item|
  result = nil
  ActiveRecord::Base.transaction do
   agent = Agent.find(773018776)
   agent.agent_events.delete_all; agent.agent_grants.delete_all; agent.agent_credentials.delete_all
   agent.update_columns(status:"idle", daily_message_cap:item[:setup][:cap], owner_id:127326141)
   agent.agent_credentials.create!(name:"HTTP contract", created_by_id:127326141, token_digest:AgentCredential.digest(secret), token_last_four:AgentCredential.digest(secret).first(4))
   ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=#{base-1} WHERE name='messages'")
   Message.create!(room_id:room,creator_id:127326141,markdown_source:"Source α & β",client_message_id:"wire-source")
   Message.create!(room_id:room,creator_id:394959859,markdown_source:"Agent",client_message_id:"wire-agent",streaming:!!item[:setup][:stream])
   ActiveRecord::Base.connection.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,parent_message_id,last_activity_at,created_at,updated_at) VALUES(#{thread},'Wire thread',#{room},394959859,#{base},'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
   Message.create!(room_id:room,creator_id:394959859,thread_id:thread,markdown_source:"Thread source",client_message_id:"wire-thread",streaming:!!item[:setup][:thread_stream])
   Message.where(id:base+1).update_all(streaming:true,streaming_updated_at:Time.current) if item[:setup][:stream]
   Message.where(id:base+2).update_all(streaming:true,streaming_updated_at:Time.current) if item[:setup][:thread_stream]
   ChannelThread.where(id:thread).update_all(locked_at:Time.current) if item[:setup][:locked]
   ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1900600010 WHERE name='messages'")
   ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1900600020 WHERE name='rooms'")
   Rails.cache = ActiveSupport::Cache::MemoryStore.new
   session=ActionDispatch::Integration::Session.new(Rails.application); session.host! "campfire.test"
   counts = -> { {messages:Message.count,rooms:Room.count,drive_attachments:DriveAttachment.count} }
   before=counts.call
   body=item[:body]&.to_json
   session.public_send(item[:method],item[:path],params:body,headers:{"Accept"=>"application/json","Content-Type"=>"application/json","Authorization"=>["Bearer",secret].join(" ")})
   response=session.response
   after=counts.call
   result=item.merge(body:body,status:response.status,response_body:response.body,response_headers:response.headers.slice("Content-Type","Cache-Control","Pragma","Retry-After","Location"),delta:before.transform_values.with_index { |v,i| after.values[i]-v })
   raise ActiveRecord::Rollback
  end
  result
 end
 puts JSON.pretty_generate({reference_pin:"d7c7de92",cases:results})
end
