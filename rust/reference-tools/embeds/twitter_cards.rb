require 'json'
base={post_id:'131',url:'https://x.com/jack/status/131',author_name:'jack Bauer',author_handle:'jack',author_avatar_url:'https://pbs.twimg.com/profile_images/1/avatar_200x200.jpg',text:'Hello @nasa, see https://example.com/x! #launch',posted_at:Time.at(1142974214).utc,replies:18041,reposts:124658,likes:310826,fetched_at:Time.at(1700000000).utc}
photo={'type'=>'photo','url'=>'https://pbs.twimg.com/media/a.jpg','width'=>800,'height'=>532,'alt'=>'A rocket'}
video={'type'=>'video','url'=>'https://video.twimg.com/x.mp4','thumbnail_url'=>'https://pbs.twimg.com/t.jpg','width'=>1280,'height'=>720}
quote={'url'=>'https://x.com/NASA/status/123','author_name'=>'NASA','author_handle'=>'NASA','text'=>'We go up'}
cases=[['full',base.merge(media:[photo,video],quote:quote)],['loading',{post_id:'132',url:'https://x.com/jack/status/132'}],['error',{post_id:'133',url:'https://x.com/jack/status/133',fetched_at:Time.at(1700000000).utc,fetch_error:'Post not found on X'}],['handleless error',{post_id:'134',fetch_error:'error'}],['no author',base.except(:author_name,:author_handle,:author_avatar_url).merge(url:nil)],['long',base.merge(text:'é'*481)],['lines',base.merge(text:"line\n"*12)],['no quote url',base.merge(quote:quote.except('url'))],['non-http quote url',base.merge(quote:{'url'=>'javascript:void(0)//https://x.com/i/status/123'}.merge(quote.except('url')))],['no text/counts',base.except(:text,:posted_at,:replies,:reposts,:likes)],['zero/negative counts',base.merge(replies:0,reposts:-1,likes:1_200_000)],['escaping',base.merge(post_id:'<"id>',author_name:'<script> & "name"',author_handle:'<user>',url:'https://x.com/a?x=1&y="2"',text:'<script>& @jack #x',media:[photo.merge('alt'=>'" <unsafe> &','url'=>'https://pbs.twimg.com/a?x=1&y=2')],quote:quote.merge('author_name'=>'<b>NASA</b>','author_handle'=>'"hi','text'=>'<img>'))],['media cap',base.merge(media:(1..6).map{|i|photo.merge('url'=>"https://pbs.twimg.com/#{i}.jpg")})],['missing dimensions',base.merge(media:[photo.except('width','height','alt'),video.except('width','height')])],['blank error',base.merge(fetch_error:'  ')],['date zone',base.merge(posted_at:Time.utc(2026,11,1,5,30)), 'America/New_York']]
cards=cases.map do |name,attrs,zone|
 Time.use_zone(zone||'UTC') do
 post=Twitter::Post.new(attrs)
 {name:name,attributes:post.attributes.slice('post_id','url','author_handle','author_name','author_avatar_url','text','posted_at','replies','reposts','likes','media','quote','fetched_at','fetch_error').as_json,display_handle:post.display_handle,display_name:post.display_name,view_url:post.view_url,profile_url:post.profile_url,logo_url:Icons.image_url_for(Icons.find('x')),zone:Time.zone.name,html:ApplicationController.renderer.render(partial:'twitter/posts/card',locals:{post:post})}
 end
end
# Avoid making the renderer's fetch-request side effect outbound. These are partial-byte goldens.
Twitter::Post.class_eval {def claim_fetch_request!;false;end}
containers=[[],[cards[0]],[cards[1],cards[2]],[cards[0]]].each_with_index.map do |entries,i|
 key=i==3 ? %q{quoted"<&'key} : 'ws15e-twitter-key'
 message=Message.new(id:99,client_message_id:key)
 posts=entries.map{|c|Twitter::Post.new(c[:attributes])}
 message.define_singleton_method(:twitter_posts){posts}
 {cards:entries.map{|c|c[:name]},client_id:key,html:ApplicationController.renderer.render(partial:'twitter/posts/cards',locals:{message:message})}
end
counts=[0,1,999,1000,9999,999999,18041,124658,310826,1_200_000,1_234_567_890,-1000,-1,999_500,999_999_999,1_000_000_000_000].map{|n|{number:n,html:ApplicationController.helpers.compact_count(n)}}
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:'d7c7de92',cards:cards,containers:containers,counts:counts})+"\n")
puts "WS15e X card Rails oracle: #{cards.size} cards, #{containers.size} containers, #{counts.size} counts"
