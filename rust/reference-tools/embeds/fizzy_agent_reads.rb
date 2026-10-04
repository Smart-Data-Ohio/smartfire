require 'json'
require 'net/http'
class Ws15eReadAccount
 attr_accessor :fizzy_account_id,:access_token,:reason
 def initialize; @fizzy_account_id='897362094';@access_token='fixture-owner';end
 def usable?;reason.blank?;end
 def mark_disconnected!(reason);@reason=reason;end
end
module Ws15eReadHttp
 def start(host,port,**options)
  raise 'Unexpected destination' unless host=='app.fizzy.do'
  http=Object.new
  http.define_singleton_method(:request) do |req|
   Thread.current[:calls] << {method:req.method,path:req.path}
   status=Thread.current[:status]
   body=case req.path
   when /columns/ then [{'id'=>'col','name'=>'Working'}]
   when /boards\/board/ then {'id'=>'board','name'=>'Engineering'}
   when /boards.json/ then [{'id'=>'board','name'=>'Engineering'}]
   else {'number'=>579}
   end
   body={'message'=>'Denied'} if status>=400
   response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'Fixture')
   response.define_singleton_method(:body) {JSON.generate(body)}
   response
  end
  yield http
 end
end
Net::HTTP.singleton_class.prepend(Ws15eReadHttp)
cases=[['boards','boards',{}],['other_account','boards',{account_id:'other'}],['board','board',{board_id:'board'}],['search','search_cards',{query:' hi ~ & 中文'}],['card','card',{account_id:'897362094',number:'000579'}],['invalid_id','board',{board_id:'../my'}],['missing_query','search_cards',{query:'  '}],['invalid_number','card',{account_id:'bad/id',number:'-1'}],['invalid_account','card',{account_id:'../my',number:1}], *%w[forbidden missing_owner no_account disconnected].map { |name| [name,'boards',{}] }, *[401,403,404,422,500].map { |status| ["status_#{status}",'boards',{},status] }]
vectors=cases.map do |name,operation,fields,status|
 Thread.current[:calls]=[];Thread.current[:status]=status || 200
 account=Ws15eReadAccount.new;account.reason='Disconnected' if name=='disconnected'
 owner=Struct.new(:fizzy_connected_account).new(name=='no_account' ? nil : account)
 agent=Struct.new(:owner,:allowed).new(name=='missing_owner' ? nil : owner,name!='forbidden')
 agent.define_singleton_method(:can?) { |cap,room| raise 'wrong scope' unless cap==:fizzy && room.nil?; allowed }
 result=Agents::FizzyReads.public_send(operation,agent:agent,**fields)
 {name:name,operation:operation,fields:fields,status:Rack::Utils.status_code(result.status),payload:result.payload,error:result.error,body:result.ok? ? result.payload : result.failure_body,reason:account.reason,calls:Thread.current[:calls]}
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),reads:vectors})+"\n")
puts "WS15e Fizzy agent reads Rails oracle: #{vectors.size} service cases"
