require 'json'
Current.user=User.find(127326141)
room=Room.find(699448326)
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
messages=[];posts=[];cases=[]
[
 [{post_id:'90071992547409931',url:'https://x.com/author/status/90071992547409931',author_name:'Author <&>',author_handle:'author',text:'Hello @nasa #launch https://example.test/a?x=1&y=2',posted_at:Time.current,replies:124658,reposts:18041,likes:310826,media:[{'type'=>'photo','url'=>'https://pbs.twimg.com/photo.jpg','width'=>800,'height'=>532,'alt'=>'Rocket <&>'}],quote:{'url'=>'https://x.com/nasa/status/3','author_name'=>'NASA','author_handle'=>'nasa','text'=>'Quote <&>'},fetched_at:Time.current}],
 [{post_id:'00022',url:'https://x.com/loader/status/00022'}, {post_id:'9',url:'https://x.com/i/status/9',fetch_error:'Missing <&>',fetched_at:Time.current}],
 [{post_id:'9999999999999999999999999',url:nil,author_name:'   ',text:'cached post',fetched_at:Time.current}]
].each_with_index do |attrs,i|
 m=room.messages.create!(creator:Current.user,markdown_source:'X preload reference',client_message_id:"twitter-preload-#{i}")
 attrs.each do |a|
  post=Twitter::Post.create!(a)
  Twitter::PostReference.create!(message:m,post:)
  posts<<post
 end
 messages<<m
 cases<<{label:"case-#{i}",message_id:m.id,html:renderer.render(partial:'twitter/posts/cards',locals:{message:Message.with_rendering_details.find(m.id)})}
end
ids=messages.map(&:id).join(',');pids=posts.map(&:id).join(',')
selects={'twitter_posts'=>"id IN (#{pids})",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'twitter_post_references'=>"message_id IN (#{ids})"}
rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:,cases:)+"\n")
puts "WS8bm2 X preload Rails oracle: #{cases.size} populated containers; #{posts.size} persisted posts"
