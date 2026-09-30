# Contracts from our pinned Rails client, URL extractor and content-free card container.
require 'net/http'
require 'json'
raise 'Wrong reference' unless Digest::SHA256.file(Rails.root.join('app/models/fizzy/client.rb')).hexdigest == ARGV.fetch(1)
TOKEN='fixture-fizzy-token'
module FizzyFakeHttp
  def start(host, port, **options)
    raise 'Unexpected network destination' unless host == 'app.fizzy.do'
    Thread.current[:options] = options.merge(write_timeout: Net::HTTP.new(host).write_timeout)
    http=Object.new
    http.define_singleton_method(:request) do |request|
      Thread.current[:request]={method:request.method,path:request.path,body:request.body,headers:request.to_hash.reject { |key,_| key=='authorization' }}
      status,body=Thread.current[:response] || [200,'{}']
      response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'Fixture')
      response.define_singleton_method(:body) { body }
      response
    end
    yield http
  end
end
Net::HTTP.singleton_class.prepend(FizzyFakeHttp)
client=Fizzy::Client.new(token:TOKEN)
operations=[['identity',[]],['boards',['acc']],['board',['acc','board']],['columns',['acc','board']],['card',['acc',12]],['search',['acc',' hi ~ & 中文']],['create_card',['acc','board'],{title:'Title',description:'Body'}],['create_comment',['acc',12],{body:'Comment'}],['move_to_column',['acc',12],{column_id:'col'}],['close_card',['acc',12]],['reopen_card',['acc',12]]]
requests=operations.map do |method,args,kwargs|
  value=client.public_send(method,*args,**(kwargs || {}))
  {method:method,args:args,kwargs:kwargs,request:Thread.current[:request],options:Thread.current[:options],value:value}
end
responses=[[200,''],[201,'not JSON'],[200,'[]'],[200,'null'],[401,'{}'],[404,'{}'],[403,'{"message":"Denied @[17]\n next"}'],[422,'{"error":"invalid"}'],[422,'{"message":"","error":"ignored"}'],[422,'{"errors":["one","two"]}'],[403,'[]'],[403,'false'],[403,'"message error"'],[403,'123'],[422,'{"errors":{"z":"last","a":"first"}}'],[500,'{"error":"secret body"}'],*[99,100,101].map { |depth| [200,('['*depth)+'0'+(']'*depth)] }].map do |status,body|
  Thread.current[:response]=[status,body]
  begin
    {status:status,body:body,value:client.card('acc',12)}
  rescue Fizzy::Client::Error => error
    {status:status,body:body,error: error.class.name.demodulize,message:error.message}
  end
end
urls=[['https://app.fizzy.do','https://app.fizzy.do/acc/cards/12 https://app.fizzy.do/acc/cards/12#comment https://app.fizzy.do/acc/cards/13abc https://app.fizzy.do/acc/cards/0'],['http://fizzy.example.test:3000','http://fizzy.example.test:7777/ABC_1/cards/2 http://fizzy.example.test/ABC_1/cards/3'],['https://app.fizzy.do','https://APP.fizzy.do/a/cards/1 https://app.fizzy.do/a/cards/1_ https://app.fizzy.do/a/cards/1é https://app.fizzy.do/a/cards/2/steps'],['bad base','https://app.fizzy.do/a/cards/1 https://app.fizzy.do:443/a/cards/2']].map do |base,text|
  ENV['FIZZY_API_BASE_URL']=base
  {base:base,text:text,refs:Fizzy::CardUrl.extract(text).map(&:to_h)}
end
ENV.delete('FIZZY_API_BASE_URL')
containers=[[],[['z',2],['a',3],['a',1]]].map do |pairs|
  message=Message.new(id:99,room_id:42,client_message_id:'ws15e-fizzy-key')
  cards=pairs.each_with_index.map { |(account,number),i| Fizzy::Card.new(id:i+1,account_id:account,number:number) }
  message.define_singleton_method(:fizzy_cards) { cards }
  {cards:cards.map { |c| c.attributes.slice('id','account_id','number') },html:ApplicationController.renderer.render(partial:'fizzy/cards/cards',locals:{message:message})}
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:'d7c7de92',requests:requests,responses:responses,urls:urls,containers:containers,sweep:{interval:Periodic::Runner::AGENT_SWEEP_INTERVAL.to_i,age:Fizzy::PerformAgentActionJob::STUCK_CLAIM_AFTER.to_i},account:{token:TOKEN,ciphertext:ActiveRecord::Encryption.encryptor.encrypt(TOKEN),secret_key_base:Rails.application.secret_key_base}},max_nesting:false)+"\n")
puts "WS15e Fizzy Rails oracle: #{requests.size} requests, #{responses.size} responses, #{urls.size} URL cases, #{containers.size} content-free containers"
