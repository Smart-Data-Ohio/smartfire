require 'json'
url='https://www.linkedin.com/feed/update/urn:li:activity:7123456789012345678'
cases=[['full',{title:'Title',description:'Description',image_url:'https://example.test/hero.png'},url],['title',{title:'Title'},url],['description',{description:'Only excerpt'},'https://www.linkedin.com/posts/smart-data_post'],['chip',{},url],['image_only',{image_url:'https://example.test/hero.png'},url],['escaping',{title:'Tom & Jerry <3 "hi"',description:"<script>untrusted</script>\n& 'quotes'",image_url:'https://example.test/a?x=1&y=2'},url],['own_fragment',{title:'Own link'},url+'?own=1#raw'],['blank',{title:'  ',description:"\t",image_url:nil},url],['share',{description:'A shared post'},'https://www.linkedin.com/feed/update/urn:li:share:9007199254740993'],['ugc',{title:'UGC'},'https://www.linkedin.com/feed/update/urn:li:ugcPost:9007199254740995']]
cards=cases.map do |name,attrs,raw|
 embed=LinkEmbed.new(**attrs,normalized_url:url,expires_at:1.hour.from_now)
 reference=LinkEmbedReference.new(id:1,link_embed:embed,url:raw)
 {name:name,attributes:attrs,url:raw,player_url:Linkedin::PostUrl.embed_url_for(raw),html:ApplicationController.renderer.render(partial:'linkedin/posts/card',locals:{reference:reference})}
end
containers=[[],[cards[0]],[cards[1],cards[2]],[cards[0]]].each_with_index.map do |cards,index|
 key=index==3 ? %q{quoted"<&'key} : 'ws15e-linkedin-key'
 message=Message.new(id:99,client_message_id:key)
 refs=cards.each_with_index.map { |card,i| LinkEmbedReference.new(id:i+1,position:i,link_embed:LinkEmbed.new(**card[:attributes],normalized_url:card[:url],expires_at:1.hour.from_now),url:card[:url]) }
 message.define_singleton_method(:link_embed_references) {refs}
 {cards:cards.map { |c| c[:name] },client_id:key,html:ApplicationController.renderer.render(partial:'linkedin/posts/cards',locals:{message:message})}
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),cards:cards,containers:containers})+"\n")
puts "WS15e LinkedIn Rails oracle: #{cards.size} cards, #{containers.size} containers"
