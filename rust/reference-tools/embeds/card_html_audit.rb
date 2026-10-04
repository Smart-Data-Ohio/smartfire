# Render every WS15e-owned card family with Rails, including JSON coercion and unsafe strings.
require 'json'
raw=%q{https://example.com/path?x=1&y="2"#<'raw>}
attrs=[['empty',{}],['image only',{image_url:'https://example.com/a?x=1&y=2'}],['blank Unicode',{title:"\u00a0\u2003",description:"\t\r\n",site_name:' ',image_url:' '}],['unsafe',{title:'<img src=x onerror=alert(1)> & "title"',description:"first\n<script>x</script> & 'quotes'",site_name:'<b>site</b>',image_url:raw}],['zero and false',{title:0,description:false,site_name:123}],['array text',{title:['one','two'],description:{'hello'=>'<world>'}}],['long Unicode',{title:'é😀'*250,description:'中'*600,site_name:'abc'*100}],['description only',{description:'Excerpt',image_url:'https://example.com/a.png'}]]
generic=[];linkedin=[]
attrs.each do |name,input|
 embed=LinkEmbed.new(**input,normalized_url:'https://example.com/shared')
 reference=LinkEmbedReference.new(id:1,link_embed:embed,url:raw)
 fields=embed.attributes.slice('title','description','site_name','image_url')
 generic << {name:name,url:raw,attributes:fields,html:ApplicationController.renderer.render(partial:'link_embeds/card',locals:{reference:reference})}
 reference.url='https://www.linkedin.com/feed/update/urn:li:activity:9007199254740999?own=1#raw'
 linkedin << {name:name,url:reference.display_url,attributes:fields,player_url:Linkedin::PostUrl.embed_url_for(reference.display_url),html:ApplicationController.renderer.render(partial:'linkedin/posts/card',locals:{reference:reference})}
end
card=Fizzy::Card.new(id:3,account_id:'897362094',number:579)
base={'title'=>'Card & <title>','board'=>{'name'=>'Board'},'tags'=>[],'steps'=>[],'assignees'=>[],'url'=>card.web_url}
cases=[['empty',{},nil],['false payload',false,nil],['whitespace error',nil,"\u00a0\u2003"],['error escapes',nil,%q{<script> & 'error'}],['unsafe fields',base.merge('title'=>'<script>bad</script> & "title"','board'=>{'name'=>'<b> & board'},'column'=>{'name'=>'<i> & column'},'assignees'=>[{'name'=>'<img> & name','avatar_url'=>'https://avatars.example/a?x=1&y=2'}],'tags'=>[%q{<'unsafe'>},'&tag'],'url'=>'javascript:alert(1)'),nil],['Ruby truthiness',base.merge('closed'=>0,'postponed'=>true,'has_more_assignees'=>0,'steps'=>[{'completed'=>0},{'completed'=>''},{'completed'=>false},{'completed'=>nil}]),nil],['Ruby collection labels',base.merge('title'=>['one','two'],'board'=>{'name'=>{'a'=>'<b>'}},'column'=>{'name'=>['in','progress']},'tags'=>[{'a'=>'<b>'},['one','two'],false,12]),nil],['avatars schemes',base.merge('assignees'=>['https://avatars.example/a.png','HTTPs://avatars.example/a.png','http://avatars.example/a.png','//avatars.example/a.png','javascript:alert(1)','https://'].map{|url|{'name'=>'name','avatar_url'=>url}}),nil],['nil collections',base.merge('tags'=>nil,'steps'=>nil,'assignees'=>nil),nil],['zero title',base.merge('title'=>0),nil],['false title',base.merge('title'=>false),nil],['date zone',base.merge('last_active_at'=>'2026-11-01T05:30:00Z'),nil,'America/New_York'],['date local',base.merge('last_active_at'=>'2026-11-01 01:30:00'),nil,'America/New_York'],['date only',base.merge('last_active_at'=>'2026-03-02'),nil,'America/New_York']]
fizzy=cases.map do |name,payload,error,zone|
 Time.use_zone(zone||'UTC') do
 cache=Fizzy::CardCache.new(payload:payload,fetch_error:error)
 {name:name,payload:payload,error:error,zone:Time.zone.name,html:ApplicationController.renderer.render(template:'rooms/fizzy/cards/show',layout:false,assigns:{card:card,cache:cache,connect_required:false,frame_id:%q{card"<&'frame}})}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),generic:generic,linkedin:linkedin,fizzy:fizzy})+"\n")
puts "WS15e card HTML Rails audit: #{generic.size} generic, #{linkedin.size} LinkedIn, #{fizzy.size} Fizzy"
