# Pinned Rails metadata/card contracts; no DB writes, outbound requests or Redis calls.
output=ARGV.fetch(0)
cases=[
  '<meta property="og:title" content="Example Title"><meta property="og:description" content="An example description."><meta property="og:site_name" content="Example"><meta property="og:image" content="https://example.com/image.png">',
  '<title>Document Title</title><meta name="twitter:title" content="Card Title"><meta name="twitter:description" content="Card description."><meta name="twitter:image" content="/card.png">',
  '<meta property="og:image" content="http://cdn.example.com/plain.png"><meta property="og:image:secure_url" content="https://cdn.example.com/secure.png">',
  '<meta property="og:image" content="images/hero.png">',
  '<meta property="og:title" content="<img src=x onerror=alert(1)>'+('Hey! '*100)+'"><meta property="og:description" content="<b>bold</b> words">',
  '<meta property="og:title" content="Tom &amp; Jerry say &quot;hi&quot; &lt;3"><meta property="og:description" content="5 &gt; 3 &amp; 2 &lt; 4, &quot;quoted&quot;">',
  '<meta property="og:title" content="&lt;img src=x onerror=alert(1)&gt;Hi"><meta property="og:description" content="<a href=&quot;javascript:alert(1)&quot;>click</a>">',
  '<meta property="og:image" content="javascript:alert(1)">',
  '<title>Plain document title</title><meta name="description" content="Plain description">',
  '<meta property="og:title" content=""><meta property="og:title" content="Ignored duplicate"><meta name="twitter:title" content="Fallback">',
  '<meta PROPERTY="OG:TITLE" content="Uppercase value"><meta name="twitter:site" content="@author">',
  '<title>Café 😀</title><meta property="og:title" content="é😀中文">',
  '<meta property="og:title" content="&#160; hello &#160;"><meta property="og:description" content="&amp;nbsp;">',
  '<meta property="og:image" content="../next.png?x=1#photo">',
  '<meta property="og:title" content="<script>bad</script><b>good</b>">',
  '<meta property="og:title" content="&lt;3">'
]
metadata=cases.map do |html|
  base='https://example.com/blog/page'
  result=LinkEmbed::MetadataParser.parse(html,base_url:base)
  {html:html,base:base,expected:result.to_h}
end
cards=[
  {title:'Title',description:'Description',site_name:'Example',image_url:'https://example.com/image.png'},
  {title:'Title'}, {description:'Only description'}, {},
  {title:'Tom & Jerry <3 "hi"',description:'<script>untrusted</script>',site_name:'A & B',image_url:'https://example.com/a?x=1&y=2'},
  {title:'  ',description:'Text',site_name:'  ',image_url:nil}
].map do |attributes|
  embed=LinkEmbed.new(**attributes,normalized_url:'https://example.com/shared',expires_at:1.hour.from_now)
  reference=LinkEmbedReference.new(id:1,link_embed:embed,url:'https://example.com/raw#own')
  {attributes:attributes,url:reference.display_url,html:ApplicationController.renderer.render(partial:'link_embeds/card',locals:{reference:reference})}
end
containers=[[],[cards[0][:attributes]],[cards[1][:attributes],cards[2][:attributes]],[cards[0][:attributes]]].each_with_index.map do |attributes,index|
  key=index==3 ? %q{quoted"<&'key} : 'ws15e-card-key'
  message=Message.new(id:99,client_message_id:key)
  references=attributes.each_with_index.map do |attrs,index|
    embed=LinkEmbed.new(**attrs,normalized_url:'https://example.com/shared',expires_at:1.hour.from_now)
    LinkEmbedReference.new(id:index+1,position:index,link_embed:embed,url:'https://example.com/raw#own')
  end
  message.define_singleton_method(:link_embed_references) { references }
  {attributes:attributes,client_id:key,html:ApplicationController.renderer.render(partial:'link_embeds/cards',locals:{message:message})}
end
File.write(output,JSON.pretty_generate({reference:'d7c7de92',metadata:metadata,cards:cards,containers:containers})+"\n")
puts "WS15e LinkEmbed Rails vectors: #{metadata.size} metadata, #{cards.size} generic cards, #{containers.size} containers"
