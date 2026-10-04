# Full production-router Fizzy wire contracts. WebMock replaces only the remote server.
require "active_support/testing/time_helpers"
require "webmock"
include WebMock::API
extend ActiveSupport::Testing::TimeHelpers
WebMock.enable!
Rails.logger = ActiveSupport::Logger.new($stderr)
secret = "ws11api-fixture-credential"
cases=[]
tools={"create"=>"create_fizzy_card","comment"=>"comment_on_fizzy_card","move"=>"move_fizzy_card","close"=>"close_fizzy_card","reopen"=>"reopen_fizzy_card"}
valid={kind:"create",board_id:"board",title:" α & β < > ",description:" Details α & β ",external_id:"wire-action"}
add=->(name,fields,setup={}) {
 cases << {name:"rest_#{name}",method: :post,path:"/agents/fizzy/card_actions",body:fields,setup:setup}
 kind=fields[:kind]; tool=tools[kind] || "create_fizzy_card"
 cases << {name:"mcp_#{name}",method: :post,path:"/agents/mcp",body:{jsonrpc:"2.0",id:11,method:"tools/call",params:{name:tool,arguments:fields}},setup:setup}
}
tools.each_key do |kind|
 fields=case kind
 when "create" then valid
 when "comment" then {kind:kind,number:"00579",body:" Comment α & β ",external_id:"wire-action"}
 when "move" then {kind:kind,number:579,column_id:"col",external_id:"wire-action"}
 else {kind:kind,number:"0",external_id:"wire-action"}
 end
 add.call("#{kind}_success",fields)
end
[{replay:true},{replay:true,cap:0},{replay:true,expired:true},{cap:0}].each_with_index { |setup,i|add.call("replay_budget_#{i}",valid,setup) }
["no_grant","room_grant","no_owner","no_account","disconnected","corrupt"].each { |name|add.call(name,valid,{name.to_sym=>true}) }
[{}, {title:" "}, {board_id:"../bad"}, {account_id:"../bad"}, {title:"a"*501}, {description:"a"*3501}, {title:["A","B"]}, {description:{name:"x"}}, {body:"a"*3501}, {column_id:"../ignored"}, {external_id:false}, {external_id:["A","B"]}, {external_id:{name:"x"}}, {account_id:false}, {account_id:true}].each_with_index do |override,i|
 fields=i==0 ? {} : valid.merge(override)
 add.call("create_validation_#{i}",fields)
end
[false,true,[],[579],{number:579},"+1","1.0",0,"000000"].each_with_index { |number,i|add.call("number_type_#{i}",{kind:"close",number:number,external_id:"wire-action"}) }
add.call("payload_bytes",valid.merge(description:"α"*3000))
add.call("validation_before_budget",valid.merge(title:"a"*501),{cap:0})
add.call("nested_ignored",{card_action:valid})
add.call("create_ignored_extras",valid.merge(number:"wrong",body:"ignored",column_id:"wrong"))
add.call("close_ignored_extras",{kind:"close",number:579,title:"a"*501,body:"a"*3501,external_id:"wire-action"})
[true,42,1.5,[],["wire-action"],["missing","wire-action"],[["wire-action"]],[false],[nil],[{name:"x"}],{}].each_with_index do |external,i|
 add.call("replay_type_#{i}",valid.merge(external_id:external),{replay:true})
end
add.call("nil_replay",valid.merge(external_id:[nil]),{replay:true,null_external:true})
add.call("hash_without_grant",valid.merge(external_id:{name:"x"}),{no_grant:true})
add.call("hash_without_account",valid.merge(external_id:{name:"x"}),{no_account:true})
["Eastern Time (US & Canada)","Asia/Tokyo","Invalid Zone"].each_with_index { |zone,i|add.call("budget_zone_#{i}",valid,{cap:0,zone:zone}) }
[false,0,"0","false",true].each_with_index { |preference,i|add.call("inbox_preference_#{i}",valid,{preference:preference}) }
travel_to Time.utc(2026,3,2,16) do
 results=cases.map do |item|
  result=nil; execution=Rails.application.executor.run!(reset:true)
  begin
   setup=item[:setup]; agent=Agent.find(773018776)
   agent.agent_approvals.destroy_all
   ActivityItem.where(source_type:"AgentBudgetNotice",source_id:AgentBudgetNotice.where(agent_id:agent.id).select(:id)).delete_all
   AgentBudgetNotice.where(agent_id:agent.id).delete_all
   agent.agent_grants.delete_all; agent.agent_credentials.delete_all
   agent.update_columns(owner_id:setup[:no_owner] ? nil : 127326141,suspended_at:nil,daily_external_action_cap:setup[:cap])
   User.find(394959859).update_columns(time_zone:setup[:zone])
   User.find(127326141).update_columns(inbox_preferences:setup.key?(:preference) ? {"agent_approvals"=>setup[:preference]} : {})
   agent.agent_credentials.create!(name:"HTTP contract",created_by_id:127326141,token_digest:AgentCredential.digest(secret),token_last_four:AgentCredential.digest(secret).first(4))
   agent.agent_grants.create!(capability:"external_action",room_id:setup[:room_grant] ? 486777696 : nil,granted_by_id:127326141) unless setup[:no_grant]
   FizzyConnectedAccount.where(user_id:127326141).delete_all
   unless setup[:no_account]
    account=FizzyConnectedAccount.create!(user_id:127326141,fizzy_account_id:"897362094",fizzy_user_id:"owner-id",fizzy_user_name:"Fixture Owner",access_token:"fixture-owner")
    account.update_columns(disconnected_reason:"Disconnected") if setup[:disconnected]
    ActiveRecord::Base.connection.execute("UPDATE fizzy_connected_accounts SET access_token='not encrypted' WHERE id=#{account.id}") if setup[:corrupt]
   end
   agent.agent_approvals.delete_all
   AgentBudgetNotice.where(agent_id:agent.id).delete_all
   ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1900700000 WHERE name='agent_approvals'")
   if setup[:replay]
    agent.agent_approvals.create!(action:"fizzy.close",summary:"Previous",payload:'{"kind":"close"}',external_id:setup[:null_external] ? nil : "wire-action",expires_at:Time.current+1.day)
    agent.agent_approvals.update_all(expires_at:Time.current-1.second) if setup[:expired]
   end
   WebMock.reset!
   Rails.cache=ActiveSupport::Cache::MemoryStore.new
   session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
   body=item[:body]&.to_json
   session.public_send(item[:method],item[:path],params:body,headers:{"Accept"=>"application/json","Content-Type"=>"application/json","Authorization"=>["Bearer",secret].join(" ")})
   response=session.response
   credential=agent.agent_credentials.first
   approvals=agent.agent_approvals.order(:id).map { |row| {action:row.action,summary:row.summary,payload:row.payload,external_id:row.external_id,status:row.status,expires_at:row.expires_at.iso8601(3),credential_matches:row.agent_credential_id==credential.id,account_matches:row.fizzy_connected_account_id==account&.id,fizzy_user_id:row.fizzy_user_id,fizzy_user_name:row.fizzy_user_name} }
   inbox=ActivityItem.where(source_type:"AgentApproval",source_id:agent.agent_approvals.select(:id)).order(:user_id).pluck(:user_id,:event_type).map { |user,event| {user:user,event:event} }
   notices=AgentBudgetNotice.where(agent_id:agent.id).order(:id).pluck(:cap,:day).map { |cap,day| {cap:cap,day:day.iso8601} }
   calls=WebMock::RequestRegistry.instance.requested_signatures.hash.flat_map { |signature,count| Array.new(count) { {method:signature.method.to_s.upcase,path:signature.uri.request_uri} } }
   result=item.merge(body:body,status:response.status,response_body:response.body,response_headers:response.headers.slice("Content-Type","Cache-Control","Pragma","Retry-After","Location"),calls:calls,reason:FizzyConnectedAccount.find_by(user_id:127326141)&.disconnected_reason,approvals:approvals,inbox:inbox,notices:notices)
  rescue Exception => error
   warn error.full_message
   raise
  ensure
   execution.complete!
  end
  result
 end
 puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:results})
end
