# Production Rails requests with no mocks. Tokens are generated from fixture parts at request time.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
SECRET = "ws11api-fixture-credential"
CASES=[]
BASE=1900300001
def contract(name, method, path, body=nil, setup={})
  CASES << {name:name,method:method,path:path,body:body,setup:setup}
end
root="/rooms/486777696/{key}/messages"
msg="#{root}/#{BASE}"
boost="#{msg}/boosts"
contract("bot_create",:post,root,"Hello 👋!")
contract("bot_create_json_is_raw",:post,root,{message:{markdown_source:"ignored",drive_file_ids:["1AbcDefGhIjKlMnOpQrSt"]}}.to_json)
contract("bot_create_empty",:post,root," ")
contract("bot_create_board",:post,root,"hi",{board:true})
contract("bot_create_budget",:post,root,"hi",{cap:0})
contract("bot_create_grant",:post,root,"hi",{grant:"read_messages"})
contract("bot_create_nonmember",:post,"/rooms/201306877/{key}/messages","hi")
contract("bot_update_raw",:patch,msg,"<div>Hello &amp; bye</div>")
contract("bot_update_drive_ignored",:patch,msg,{message:{drive_file_ids:["2BcdEfgHiJkLmNoPqRsTu"]}}.to_json,{drive:true})
contract("bot_update_other",:patch,msg,"edited",{other_creator:true})
contract("bot_update_system",:patch,msg,"edited",{system:true})
contract("bot_update_grant",:patch,msg,"edited",{grant:"read_messages"})
contract("bot_destroy",:delete,msg)
contract("bot_destroy_other",:delete,msg,nil,{other_creator:true})
contract("bot_destroy_system",:delete,msg,nil,{system:true})
contract("bot_index_roots",:get,"#{root}?after=#{BASE}",nil,{index:true})
contract("bot_index_starter_summary",:get,"#{root}?after=#{BASE}",nil,{index:true,summary:true})
contract("bot_index_missing_cursor",:get,"#{root}?before=0")
contract("bot_index_grant",:get,root,nil,{grant:"react"})
contract("bot_index_markdown_reply_forward_drive",:get,"#{root}?after=#{BASE}",nil,{index:true,details:true})
contract("bot_boost_create",:post,boost,"👀")
contract("bot_boost_shortcode",:post,boost,":heart:")
contract("bot_boost_unknown",:post,boost,":lol:")
contract("bot_boost_icon",:post,boost,"Nice!",{icon:"openai"})
contract("bot_boost_html",:post,boost,"<>& α\u2028\u2029")
contract("bot_boost_empty",:post,boost," ")
contract("bot_boost_grant",:post,boost,"👀",{grant:"read_messages"})
contract("bot_boost_missing_message",:post,"#{root}/0/boosts","👀")
contract("bot_boost_other_room",:post,"/rooms/201306877/{key}/messages/#{BASE}/boosts","👀")
contract("bot_boost_destroy",:delete,"#{boost}/1900300010")
contract("bot_boost_destroy_other",:delete,"#{boost}/1900300011")
contract("bot_boost_destroy_missing",:delete,"#{boost}/0")
contract("bot_reply_create",:post,root,"Reply",{reply:true})
contract("bot_reply_read",:get,root,nil,{reply:true})
contract("bot_reply_update",:patch,msg,"edited",{reply:true})
contract("bot_reply_destroy",:delete,msg,nil,{reply:true})
contract("bot_reply_boost_create",:post,boost,"👀",{reply:true})
contract("bot_reply_boost_destroy",:delete,"#{boost}/1900300010",nil,{reply:true})
contract("bot_agent_boost",:post,boost,"👀",{agent_token:true})
contract("bot_agent_revoked",:post,root,"hi",{agent_token:true,revoked:true})
contract("bot_agent_expired",:get,root,nil,{agent_token:true,expired:true})
contract("bot_index_work_human_owner",:get,"#{root}?after=#{BASE}",nil,{index:true,summary:true,work:"in_progress",owner:127326141})
contract("bot_index_work_unassigned",:get,"#{root}?after=#{BASE}",nil,{index:true,summary:true,work:"planned"})
contract("bot_index_work_bot_owner",:get,"#{root}?after=#{BASE}",nil,{index:true,summary:true,work:"blocked",owner:394959859})
contract("bot_index_work_bot_owner_without_post",:get,"#{root}?after=#{BASE}",nil,{index:true,summary:true,work:"blocked",owner:394959859,grant:"read_messages"})
contract("bot_index_work_suspended_owner",:get,"#{root}?after=#{BASE}",nil,{index:true,summary:true,work:"blocked",owner:394959859,suspended:true})
contract("bot_index_work_nonmember_owner",:get,"#{root}?after=#{BASE}",nil,{index:true,summary:true,work:"done",owner:712064548})
# Exact page windows (including tied timestamps), cursor shape and reply scope.
[
 ["latest",root,{index:true,fill:45}],
 ["older","#{root}?before=#{BASE+41}",{index:true,fill:45}],
 ["newer","#{root}?after=#{BASE}",{index:true,fill:45}],
 ["end","#{root}?after=#{BASE+47}",{index:true,fill:45}],
 ["both","#{root}?before=#{BASE+41}&after=#{BASE}",{index:true,fill:45}],
 ["empty_before","#{root}?before=&after=#{BASE}",{index:true,fill:45}],
 ["ignored_limit","#{root}?after=#{BASE}&limit=2",{index:true,fill:45}],
 ["blank_cursor","#{root}?after=%20",{index:true}],
 ["numeric_suffix","#{root}?after=#{BASE}junk",{index:true}],
 ["thread_cursor","#{root}?after=#{BASE+2}",{index:true}],
 ["array","#{root}?after[]=#{BASE}",{index:true}],
 ["array_missing","#{root}?after[]=0",{index:true}],
 ["hash","#{root}?after[x]=#{BASE}",{index:true}],
 ["icons","#{root}?after=#{BASE}",{index:true,icon:"openai",room_icon:"fire"}],
 ["nonmember","/rooms/201306877/{key}/messages",{}]
].each { |name,path,setup| contract("bot_index_page_#{name}",:get,path,nil,setup) }
contract("bot_reply_stale",:post,root,"reply",{reply:true,reply_expired:true})
contract("bot_reply_tampered",:post,root,"reply",{reply:true,reply_tampered:true})
contract("bot_reply_wrong_room",:post,root,"reply",{reply:true,reply_room:201306877})
contract("bot_create_human_key",:post,root,"reply",{key:"127326141-"})
contract("bot_index_invalid_key",:get,root,nil,{key:"invalid-bot-key"})
contract("bot_create_invalid_key",:post,root,"reply",{key:"invalid-bot-key"})
contract("bot_create_empty_utf8",:post,root,"\u00a0")
contract("bot_update_empty_body",:patch,msg,"")
contract("bot_destroy_missing_message",:delete,"#{root}/0")
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776)
  bot=User.find(394959859)
  result=CASES.map do |item|
    setup=item[:setup]
    agent.agent_grants.delete_all
    agent.agent_credentials.delete_all
    agent.update_columns(daily_message_cap:setup[:cap],owner_id:127326141,suspended_at:nil)
    AgentGrant.create!(agent:agent,capability:setup[:grant],granted_by_id:127326141) if setup[:grant]
    credential=agent.agent_credentials.create!(name:"Bot contract",created_by_id:127326141,token_digest:AgentCredential.digest(SECRET),token_last_four:AgentCredential.digest(SECRET).first(4))
    credential.update_columns(revoked_at:Time.current) if setup[:revoked]
    credential.update_columns(expires_at:Time.current) if setup[:expired]
    Room.find(486777696).update_columns(type:"Rooms::Closed")
    Message.where("id >= ?",BASE).destroy_all
    ChannelThread.where(id:1900300008).delete_all
    starter=Message.create!(id:BASE,room_id:486777696,creator_id:394959859,client_message_id:"bot-contract-starter",markdown_source:"Starter")
    Boost.create!(id:1900300010,message:starter,booster_id:394959859,content:"👍")
    Boost.create!(id:1900300011,message:starter,booster_id:127326141,content:"👏")
    Boost.connection.execute("UPDATE sqlite_sequence SET seq=1900300011 WHERE name='boosts'")
    Message.connection.execute("UPDATE sqlite_sequence SET seq=#{BASE} WHERE name='messages'")
    starter.drive_attachments.create!(file_id:"1AbcDefGhIjKlMnOpQrSt") if setup[:drive]
    starter.update_columns(creator_id:127326141) if setup[:other_creator]
    starter.update_columns(system_note:true) if setup[:system]
    if setup[:index]
      next_message=Message.create!(id:BASE+1,room_id:486777696,creator_id:394959859,client_message_id:"bot-contract-next",markdown_source:"**Next**")
      thread=ChannelThread.create!(id:1900300008,name:"Excluded",room_id:486777696,creator_id:394959859, parent_message_id: setup[:summary] ? next_message.id : nil)
      Message.create!(id:BASE+2,thread:thread,room_id:486777696,creator_id:394959859,client_message_id:"bot-contract-thread",markdown_source:"Thread only")
      if setup[:details]
        next_message.update_columns(reply_to_message_id:BASE,reply_notify_author:true,forwarded_from_message_id:BASE,forwarded_at:Time.current,forward_note:"Forward note",streaming:true)
        next_message.drive_attachments.create!(file_id:"1AbcDefGhIjKlMnOpQrSt")
      end
    end
    (setup[:fill] || 0).times do |i|
      Message.create!(id:BASE+3+i,room_id:486777696,creator_id:394959859,client_message_id:"bot-contract-fill-#{i}",markdown_source:"Filler #{i}")
    end
    Room.find(486777696).update_columns(icon_name:setup[:room_icon])
    thread.update_columns(work_status:setup[:work],work_owner_id:setup[:owner]) if setup[:work]
    agent.update_columns(suspended_at:Time.current) if setup[:suspended]
    Room.find(486777696).update_columns(type:"Rooms::Board") if setup[:board]
    bot.update_columns(icon_name:setup[:icon])
    Rails.cache=ActiveSupport::Cache::MemoryStore.new
    key=setup[:reply] ? bot.reply_token_for(Room.find(setup[:reply_room] || 486777696),expires_in:setup[:reply_expired] ? -1.minute : 5.minutes) : "394959859-BenderToken1"
    key += "x" if setup[:reply_tampered]
    key=setup[:key] if setup[:key]
    key="credential-route" if setup[:agent_token]
    session=ActionDispatch::Integration::Session.new(Rails.application)
    session.host! "campfire.test"
    headers={"Accept"=>"application/json","Content-Type"=>"text/plain"}
    headers["Authorization"]=["Bearer",SECRET].join(" ") if setup[:agent_token]
    before={messages:Message.count,boosts:Boost.count}
    session.public_send(item[:method],item[:path].sub("{key}",key),params:item[:body],headers:headers)
    response=session.response
    after={messages:Message.count,boosts:Boost.count}
    state=Message.find_by(id:BASE)
    created=Message.where("id > ?",BASE).order(:id).last unless setup[:index]
    item.merge(status:response.status,response_body:response.body,response:(JSON.parse(response.body) rescue nil),response_headers:response.headers.slice("Content-Type","Cache-Control","Pragma","X-Total-Count","Link","Location","Retry-After"),delta:after.transform_values.with_index{|v,i|v-before.values[i]},state:state && {markdown_source:state.markdown_source,plain_text:state.plain_text_body,drive_ids:state.drive_attachments.pluck(:file_id)},created:created && {plain_text:created.plain_text_body,drive_ids:created.drive_attachments.pluck(:file_id)})
  end
  puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:result})
end
