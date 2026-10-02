require 'json'
require 'net/http'
ActiveJob::Base.queue_adapter=:test
client=Slack::Client.new(token:'fixture-user-grant',pacing:false)
def capture
 result=yield
 result=result.to_h if result.is_a?(Slack::MarkdownConverter::Result)
 {'result'=>result}
rescue StandardError=>e
 {'class'=>e.class.name,'message'=>e.message}
end
roots=[nil,false,true,0,1.5,[],[{}],'','ok','error','okerror',{'ok'=>true}]
client_rows=roots.map{|v|{input:v,expected:capture{client.send(:check_ok!,v,'users.list')}}}
markdown_inputs=roots+[{'text'=>[1,'x']},{'text'=>{'a'=>1}},{'attachments'=>{}},{'attachments'=>{'text'=>'hi'}},{'attachments'=>[nil]},{'attachments'=>[false]},{'attachments'=>['pretext text fallback']},{'files'=>{'name'=>'x','permalink'=>'https://example.org'}},{'files'=>[1]},{'files'=>['permalink']},{'bot_id'=>false,'attachments'=>[{'text'=>'x'}]}]
markdown_rows=markdown_inputs.map{|v|{input:v,expected:capture{Slack::MarkdownConverter.convert(v,users:{})}}}
workspace=SlackWorkspace.create!(client_id:'fixture-client',client_secret:'fixture-secret')
owner=User.create!(name:'Run owner')
run=SlackImport.create!(slack_workspace:workspace,user:owner,kind:'workspace',mode:'import')
SlackImport::Record.create!(slack_workspace:workspace,slack_import:run,slack_kind:'user',slack_key:'UKNOWN',record:owner,created_record:false)
mapper=Slack::UserMapper.new(workspace:,run:)
mapper_inputs=[nil,false,true,1,1.5,'','id',{}, {'id'=>'UFIXTURE','profile'=>false},[nil],[false],[1],[{'id'=>'UFIXTURE','profile'=>[]}],[{'id'=>'UFIXTURE','profile'=>{'real_name'=>false}}]]
mapper_inputs += [false,true,0,1.5,'',[]].map{|profile|[{'id'=>'UFRESH','profile'=>profile}]}
mapper_inputs += [[{'id'=>'UKNOWN','profile'=>[]}],[{'id'=>'','profile'=>false}],[{'id'=>'UBOT','is_bot'=>true,'profile'=>[]}]]
mapper_rows=mapper_inputs.map do |v|
 expected=capture{mapper.preview_page(v)}
 {input:v,expected:}
end
conversation_mapper=Slack::ConversationMapper.new(workspace:,run:)
conversation_rows=roots.map do |v|
 {input:v,expected:capture do
  result=conversation_mapper.preview(v,member_ids:[],users:{})
  {action:result.action,room_id:result.room&.id,reason:result.skip_reason}
 end}
end
errors=[IOError.new('fixture io'),EOFError.new('fixture eof'),SocketError.new('fixture lookup'),Net::OpenTimeout.new('execution expired'),Net::ReadTimeout.new,Net::WriteTimeout.new,Timeout::Error.new('fixture timeout'),Errno::ECONNRESET.new('fixture reset'),Errno::ECONNREFUSED.new('fixture refused'),Errno::EPIPE.new('fixture pipe'),OpenSSL::SSL::SSLError.new('fixture tls')]
transport_rows=errors.map do |error|
 attempts=0
 c=Slack::Client.new(token:'fixture-user-grant',pacing:false)
 c.define_singleton_method(:get){|*|attempts+=1;raise error}
 {input_class:error.class.name,input_message:error.message,expected:capture{c.users_list},attempts:}.tap{|r|r[:attempts]=attempts}
end
http_inputs=[[200,'null'],[400,'null'],[400,'false'],[200,'false'],[200,'1.5'],[400,'[]'],[200,'not json'],[400,'not json'],[503,'{"ok":false,"error":"invalid_auth"}'],[200,'{/* fixture */"ok":true}'],[200,'{"ok":true,// fixture\n"name":"Fixture"}']]
Net::HTTP.singleton_class.prepend(Module.new do
 define_method(:start) do |*args,**opts,&block|
  fake=Object.new
  fake.define_singleton_method(:get) do |*|
   status,body=Thread.current[:ws16_http_input]
   response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'Fixture')
   response.instance_variable_set(:@read,true);response.body=body;response
  end
  block.call(fake)
 end
end)
http_rows=http_inputs.map do |status,body|
 Thread.current[:ws16_http_input]=[status,body.gsub('\\n',"\n")]
 attempts=0
 c=Slack::Client.new(token:'fixture-user-grant',pacing:false,on_request:->(*){attempts+=1})
 c.define_singleton_method(:pause){|*|}
 expected=capture{c.users_list}
 {status:,body:Thread.current[:ws16_http_input][1],expected:,attempts:}
end
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/payloads.json'),JSON.pretty_generate({client:client_rows,markdown:markdown_rows,mapper:mapper_rows,conversation:conversation_rows,transport:transport_rows,http:http_rows})+"\n")
puts "Slack payload edges: #{client_rows.size} client, #{markdown_rows.size} converter, #{mapper_rows.size} mapper, #{conversation_rows.size} conversation, #{transport_rows.size} transport, #{http_rows.size} HTTP cases generated"
