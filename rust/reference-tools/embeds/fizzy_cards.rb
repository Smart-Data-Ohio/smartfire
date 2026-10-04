# Run in the plain pinned reference image. These are rendered by OUR Rails views, never handwritten HTML.
require 'json'
card=Fizzy::Card.new(id:3,account_id:'897362094',number:579)
base={ 'title'=>'Fix <billing> & "quotes"', 'board'=>{'name'=>'Engineering'}, 'column'=>{'name'=>'In Progress'}, 'url'=>'https://app.fizzy.do/897362094/cards/579', 'assignees'=>[{'name'=>'David', 'avatar_url'=>'https://cdn.example.test/avatar.png'},{'name'=>'<Bad>', 'avatar_url'=>'http://bad.example/avatar.png'}], 'has_more_assignees'=>true, 'tags'=>['billing','<urgent>'], 'steps'=>[{'completed'=>true},{'completed'=>false}], 'last_active_at'=>'2026-03-02T15:00:00Z' }
cases=[['connect',true,nil,nil],['loading',false,nil,nil],['missing',false,nil,'not_found'],['error',false,nil,'Fizzy returned 500 <danger>'],['full',false,base,nil],['stale_error',false,base,'error ignored with payload'],['closed',false,base.merge('closed'=>true),nil],['postponed',false,base.merge('postponed'=>true,'column'=>nil),nil],['triage',false,base.merge('column'=>nil,'assignees'=>[],'tags'=>[],'steps'=>[],'has_more_assignees'=>false,'last_active_at'=>'bad date'),nil],['no_title',false,base.merge('title'=>nil,'url'=>'javascript:alert(1)'),nil],['empty_payload',false,{},nil]]
result=cases.map do |name,connect,payload,error|
  cache=Fizzy::CardCache.new(payload:payload,fetch_error:error)
  html=ApplicationController.renderer.render(template:'rooms/fizzy/cards/show',layout:false,assigns:{card:card,cache:cache,connect_required:connect,frame_id:'card_for_message_99_fizzy_card_3'})
  {name:name,connect:connect,payload:payload,error:error,html:html}
end
serialized=[nil,false,{},["one","two"]].map { |payload| {payload:payload,stored:Fizzy::CardCache.new(payload:payload).attributes_for_database['payload']} }
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),card:card.attributes.slice('id','account_id','number'),frames:result,serialized_payloads:serialized})+"\n")
puts "WS15e Fizzy card Rails oracle: #{result.size} frame branches, #{serialized.size} cache serialization cases"
