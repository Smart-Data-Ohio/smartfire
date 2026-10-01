# Real multipart attachment requests and signed-input errors; each request commits.
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
def multipart(attachment_name, client=nil)
 body="--ws11api-attachment\r\nContent-Disposition: form-data; name=\"#{attachment_name}\"; filename=\"note.txt\"\r\nContent-Type: text/plain\r\n\r\nAttachment α & β\r\n"
 body += "--ws11api-attachment\r\nContent-Disposition: form-data; name=\"message[client_message_id]\"\r\n\r\n#{client}\r\n" if client
 body+"--ws11api-attachment--\r\n"
end
[false,true].each do |bot|
 path=bot ? "/rooms/#{room}/394959859-BenderToken1/messages" : "/rooms/#{room}/agents/messages"
 field=bot ? "attachment" : "message[attachment]"
 [[{},nil],[{repeat:1},"attachment-replay"],[{message_cap:0},nil],[{grant:["read_messages"]},nil],[{board:true},nil]].each_with_index do |(setup,client),i|
  add.call("attachment_#{bot ? 'bot' : 'rest'}_#{i}",:post,path,multipart(field,client || "attachment-#{bot}-#{i}"),setup.merge(content_type:"multipart/form-data; boundary=ws11api-attachment",attachment_case:true,bot_key:bot))
 end
end
["invalid-signature",nil,"",false,3,[],{}].each_with_index do |attachment,i|
 add.call("attachment_signed_#{i}",:post,"/rooms/#{room}/agents/messages",{message:{attachment:attachment,client_message_id:"attachment-signed-#{i}"}},{attachment_case:true})
end
# Multipart thread and Drive checks use the same production Posting service.
def with_fields(body, fields)
 fields.each do |name,value|
  chunk="--ws11api-attachment\r\nContent-Disposition: form-data; name=\"#{name}\"\r\n\r\n#{value}\r\n"
  body=body.sub("--ws11api-attachment--\r\n",chunk+"--ws11api-attachment--\r\n")
 end
 body
end
[
 [{"thread_id"=>thread},{}],
 [{"thread_id"=>thread},{locked:true}],
 [{"thread_id"=>0},{}],
 [{"thread_id"=>thread},{board:true}],
 [{"message[drive_file_ids][]"=>"1AbcDefGhIjKlMnOpQrSt"},{}],
 [{"message[drive_file_ids][]"=>"bad/id"},{}],
 [{},{grant:["post_messages"],revoked:true}],
 [{},{grant:["post_messages"],grant_room:201306877}],
 [{},{remove_member:true}]
].each_with_index do |(fields,setup),i|
 add.call("attachment_extended_rest_#{i}",:post,"/rooms/#{room}/agents/messages",with_fields(multipart("message[attachment]","attachment-extended-#{i}"),fields),setup.merge(content_type:"multipart/form-data; boundary=ws11api-attachment",attachment_case:true))
end
[
 [{},{grant:["react"]}],
 [{},{grant:["post_messages"],revoked:true}],
 [{},{grant:["post_messages"],grant_room:201306877}],
 [{},{remove_member:true}],
 [{"thread_id"=>thread},{}]
].each_with_index do |(fields,setup),i|
 add.call("attachment_bot_extended_#{i}",:post,"/rooms/#{room}/394959859-BenderToken1/messages",with_fields(multipart("attachment","bot-extended-#{i}"),fields),setup.merge(content_type:"multipart/form-data; boundary=ws11api-attachment",attachment_case:true,bot_key:true))
end
[
 [{},{message_cap:0}],
 [{reply_to_message_id:0},{}],
 [{drive_file_ids:["bad/id"]},{}],
 [{drive_file_ids:false},{}],
 [{},{grant:["read_messages"]}],
 [{},{remove_member:true}],
 [{thread_id:thread},{locked:true}]
].each_with_index do |(attrs,setup),i|
 thread_id=attrs.delete(:thread_id)
 body={message:{attachment:"invalid-signature",client_message_id:"attachment-precedence-#{i}"}.merge(attrs)}
 body[:thread_id]=thread_id if thread_id
 add.call("attachment_precedence_#{i}",:post,"/rooms/#{room}/agents/messages",body,setup.merge(attachment_case:true))
end
travel_to Time.utc(2026,3,2,16) do
 results=cases.map do |item|
  result=nil
  execution=Rails.application.executor.run!(reset:true)
  begin
   begin
    Message.where("id >= ?",base).destroy_all
    ThreadTag.where(channel_thread_id:thread).delete_all
    ChannelThread.where(id:thread).delete_all
    Room.where(id:room).update_all(type:"Rooms::Closed")
    Membership.find_or_create_by!(room_id:room,user_id:394959859)
    agent=Agent.find(773018776)
    User.find(394959859).update_columns(bot_token_digest:User.digest_bot_token("BenderToken1"))
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
    agent.update_columns(daily_message_cap:item[:setup][:message_cap])
    Rails.cache=ActiveSupport::Cache::MemoryStore.new
    session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
    body=item[:body].is_a?(String) ? item[:body] : item[:body]&.to_json
    headers={"Accept"=>"application/json","Content-Type"=>(item[:setup][:content_type] || "application/json"),"Authorization"=>["Bearer",secret].join(" ")}
    headers.delete("Authorization") if item[:setup][:bot_key]
    (item[:setup][:repeat] || 0).times {session.public_send(item[:method],item[:path],params:body,headers:headers)}
    session.public_send(item[:method],item[:path],params:body,headers:headers)
    response=session.response
    created=Message.where("id >= ?",base+3).order(:id).last
    blob=created&.attachment&.blob
    attachment_state={messages:Message.where("id >= ?",base+3).count,attachment:blob && {filename:blob.filename.to_s,content_type:blob.content_type,byte_size:blob.byte_size,checksum:blob.checksum},markdown_source:created&.markdown_source}
    result=item.merge(attachment_state:attachment_state,body:body,status:response.status,response_body:response.body,response_headers:%w[Content-Type Cache-Control Pragma Retry-After Location].to_h { |key| [key,response.headers[key]] })

   end
  ensure
   execution.complete!
  end
  result
 end
 puts JSON.pretty_generate({reference_pin:"d7c7de92",cases:results})
end
