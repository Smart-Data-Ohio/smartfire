# Full production-router Fizzy wire contracts. WebMock replaces only the remote server.
require "active_support/testing/time_helpers"
require "webmock"
include WebMock::API
extend ActiveSupport::Testing::TimeHelpers
WebMock.enable!
# WebMock normalizes CGI '+' to '%20' in its registry. Record the Net::HTTP
# request before that normalization so Rust must send Rails' actual target bytes.
$fizzy_requests=[]
Net::HTTPGenericRequest.prepend(Module.new do
 def initialize(*args, **kwargs)
  super
  $fizzy_requests << {method:method,path:path}
 end
end)
Rails.logger = ActiveSupport::Logger.new($stderr)
secret = "ws11api-fixture-credential"
cases = []
reads = [
 ["boards", "list_fizzy_boards", {}, "/agents/fizzy/boards"],
 ["board", "get_fizzy_board", {board_id:"board"}, "/agents/fizzy/boards/board"],
 ["search", "search_fizzy_cards", {q:" hi ~ & 中文"}, "/agents/fizzy/cards/search?q=+hi+~+%26+%E4%B8%AD%E6%96%87"],
 ["card", "get_fizzy_card", {account_id:"897362094",number:"00579"}, "/agents/fizzy/cards/897362094/00579"]
]
add = ->(name, tool, args, path, setup={}) {
 cases << {name:"rest_#{name}",method: :get,path:path,body:nil,setup:setup}
 cases << {name:"mcp_#{name}",method: :post,path:"/agents/mcp",body:{jsonrpc:"2.0",id:11,method:"tools/call",params:{name:tool,arguments:args}},setup:setup}
}
reads.each { |name,tool,args,path| add.call(name,tool,args,path) }
[401,403,404,422,429,500].each { |status| add.call("status_#{status}", "list_fizzy_boards", {}, "/agents/fizzy/boards", {remote_status:status}) }
["no_grant","room_grant","no_owner","no_account","disconnected","corrupt"].each { |name| add.call(name,"list_fizzy_boards",{},"/agents/fizzy/boards",{name.to_sym=>true}) }
add.call("invalid_account","list_fizzy_boards",{account_id:"../bad"},"/agents/fizzy/boards?account_id=..%2Fbad")
add.call("other_account","list_fizzy_boards",{account_id:"other"},"/agents/fizzy/boards?account_id=other")
add.call("invalid_number","get_fizzy_card",{account_id:"897362094",number:"1e3"},"/agents/fizzy/cards/897362094/1e3")
add.call("invalid_board","get_fizzy_board",{board_id:".."},"/agents/fizzy/boards/..")
add.call("blank_query","search_fizzy_cards",{q:" "},"/agents/fizzy/cards/search?q=+")
[false,true,[],["a","b"],{name:"x"},0].each_with_index do |query,i|
 add.call("query_type_#{i}","search_fizzy_cards",{q:query},"/agents/fizzy/cards/search?#{ {q:query}.to_query }")
end
travel_to Time.utc(2026,3,2,16) do
 results=cases.map do |item|
  result=nil; execution=Rails.application.executor.run!(reset:true)
  begin
   setup=item[:setup]; agent=Agent.find(773018776)
   agent.agent_grants.delete_all; agent.agent_credentials.delete_all
   agent.update_columns(owner_id:setup[:no_owner] ? nil : 127326141,suspended_at:nil)
   agent.agent_credentials.create!(name:"HTTP contract",created_by_id:127326141,token_digest:AgentCredential.digest(secret),token_last_four:AgentCredential.digest(secret).first(4))
   agent.agent_grants.create!(capability:"fizzy",room_id:setup[:room_grant] ? 486777696 : nil,granted_by_id:127326141) unless setup[:no_grant]
   FizzyConnectedAccount.where(user_id:127326141).delete_all
   unless setup[:no_account]
    account=FizzyConnectedAccount.create!(user_id:127326141,fizzy_account_id:"897362094",fizzy_user_id:"owner-id",access_token:"fixture-owner")
    account.update_columns(disconnected_reason:"Disconnected") if setup[:disconnected]
    ActiveRecord::Base.connection.execute("UPDATE fizzy_connected_accounts SET access_token='not encrypted' WHERE id=#{account.id}") if setup[:corrupt]
   end
   WebMock.reset!; $fizzy_requests.clear
   status=setup[:remote_status] || 200
   remote_body=status>=400 ? '{"message":"Denied"}' : '[{"id":"board","name":"Engineering α & β < >","extra":{"first":1,"second":2}}]'
   stub_request(:get,%r{https://app.fizzy.do/(897362094|other)/boards.json}).to_return(status:status,body:remote_body)
   stub_request(:get,"https://app.fizzy.do/897362094/boards/board.json").to_return(body:'{"id":"board","name":"Engineering α & β < >"}')
   stub_request(:get,"https://app.fizzy.do/897362094/boards/board/columns.json").to_return(body:'[{"id":"col","name":"Working"}]')
   stub_request(:get,%r{https://app.fizzy.do/897362094/search.json}).to_return(body:'{"number":579,"title":"α & β < >"}')
   stub_request(:get,"https://app.fizzy.do/897362094/cards/579.json").to_return(body:'{"number":579,"title":"α & β < >"}')
   Rails.cache=ActiveSupport::Cache::MemoryStore.new
   session=ActionDispatch::Integration::Session.new(Rails.application);session.host! "campfire.test"
   body=item[:body]&.to_json
   session.public_send(item[:method],item[:path],params:body,headers:{"Accept"=>"application/json","Content-Type"=>"application/json","Authorization"=>["Bearer",secret].join(" ")})
   response=session.response
   calls=$fizzy_requests.dup
   result=item.merge(body:body,status:response.status,response_body:response.body,response_headers:response.headers.slice("Content-Type","Cache-Control","Pragma","Retry-After","Location"),calls:calls,reason:FizzyConnectedAccount.find_by(user_id:127326141)&.disconnected_reason)
  ensure
   execution.complete!
  end
  result
 end
 puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:results})
end
