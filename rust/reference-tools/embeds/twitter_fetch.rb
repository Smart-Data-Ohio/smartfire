require 'json'
require 'net/http'
base={ 'id'=>'424242','text'=>'just setting up my twttr','created_timestamp'=>1142974214,'likes'=>310826,'retweets'=>124658,'replies'=>18041,'author'=>{'name'=>'jack','screen_name'=>'jack','avatar_url'=>'https://pbs.twimg.com/profile_images/1/avatar_200x200.jpg'} }
photo=->(url,extra={}){{'type'=>'photo','url'=>url}.merge(extra)}
cases=[
 ['success',base],
 ['photos',base.merge('media'=>{'photos'=>[photo.call('https://pbs.twimg.com/media/a.jpg?name=orig',{'width'=>800,'height'=>532}),photo.call('https://pbs.twimg.com/media/b.jpg',{'width'=>640,'height'=>480,'altText'=>'A desk'})]})],
 ['video',base.merge('media'=>{'photos'=>[photo.call(nil)],'videos'=>[{'type'=>'video','url'=>'https://video.twimg.com/1/v.mp4?tag=16','thumbnail_url'=>'https://pbs.twimg.com/1/t.jpg','width'=>1280,'height'=>720}]})],
 ['quote',base.merge('quote'=>{'url'=>'https://x.com/NASA/status/123','text'=>'We go up','author'=>{'name'=>'NASA','screen_name'=>'NASA'},'media'=>{'photos'=>[photo.call('https://pbs.twimg.com/q.jpg')]}})],
 ['html',base.merge('text'=>'hi <img src=x onerror=alert(1)> there','author'=>base['author'].merge('name'=>'<b>jack</b>'))],
 ['evil hosts',base.merge('author'=>base['author'].merge('avatar_url'=>'https://evil.example/avatar.jpg'),'media'=>{'photos'=>[photo.call('https://evil.example/pic.jpg')],'videos'=>[{'type'=>'video','url'=>'https://evil.example/v.mp4','thumbnail_url'=>'https://pbs.twimg.com/t.jpg'}]})],
 ['alt limits',base.merge('media'=>{'photos'=>[photo.call('https://pbs.twimg.com/a.jpg',{'altText'=>'<b>hi</b>'}),photo.call('https://pbs.twimg.com/b.jpg',{'altText'=>'y'*2000})]})],
 ['media cap/coercion',base.merge('media'=>{'photos'=>[nil,'bad',{'type'=>'gif','url'=>'https://video.twimg.com/g.mp4','thumbnail_url'=>'https://pbs.twimg.com/g.jpg','width'=>'0x10','height'=>'08','altText'=>' ','alt'=>'fallback'}]+(1..6).map{|n|photo.call("https://pbs.twimg.com/#{n}.jpg",{'width'=>2.9,'height'=>'2_000'})}},'likes'=>'123','replies'=>2.9,'retweets'=>-3)],
 ['scheme case',base.merge('author'=>base['author'].merge('avatar_url'=>'HTTPs://pbs.twimg.com/a.jpg'),'media'=>{'photos'=>[photo.call('HTTPS://pbs.twimg.com/a.jpg')],'videos'=>[{'type'=>'video','url'=>'HTTPS://video.twimg.com/a.mp4','thumbnail_url'=>'HTTPs://pbs.twimg.com/t.jpg'}]})],
 ['twimg strict',base.merge('author'=>{'screen_name'=>' jack ','name'=>false,'avatar_url'=>'https://PBS.TWIMG.COM/a.jpg'},'media'=>{'photos'=>[photo.call('http://pbs.twimg.com/a'),photo.call('https://pbs.twimg.com.evil/a'),photo.call('https://pbs.twimg.com/a b'),photo.call('https://user:pass@pbs.twimg.com:444/a?x=1#f')],'videos'=>[{'type'=>'gif','url'=>'https://video.twimg.com/a','thumbnail_url'=>nil}]})],
 ['empty/coercion',{'id'=>'','text'=>false,'author'=>{'screen_name'=>'not.a.handle','name'=>['a',1]},'quote'=>{'text'=>'<b></b>'},'created_at'=>'bad','likes'=>false}],
 ['entities/script',base.merge('text'=>'Tom &amp; Jerry<script>bad</script><style>css</style> &lt;b&gt;\u00a0','author'=>{'screen_name'=>'abcdefghijklmnop','name'=>' &quot;name&quot; '})],
 ['text limit',base.merge('text'=>'é'*4001)],
 ['float timestamp',base.merge('created_timestamp'=>1142974214.123456)],
 ['created_at',base.except('created_timestamp').merge('created_at'=>'Tue Mar 21 20:50:14 +0000 2006')],
 ['iso date',base.except('created_timestamp').merge('created_at'=>'2026-09-30T10:22:33.123456-04:00')],
 ['no date',base.except('created_timestamp')]
]
fields=%w[url author_handle author_name author_avatar_url text posted_at replies reposts likes media quote fetch_error]
rows=cases.map do |name,tweet|
 post=Twitter::Post.create!(post_id:'424242',url:'https://x.com/jack/status/424242')
 fetcher=Twitter::PostFetcher.new(post)
 body={code:200,tweet:tweet}.to_json
 fetcher.define_singleton_method(:get){|path| @oracle_path=path; [Net::HTTPOK.new('1.1','200','OK'),body]}
 fetcher.fetch
 result=post.reload.attributes.slice(*fields)
 result['posted_at']=post.posted_at&.utc&.strftime('%Y-%m-%d %H:%M:%S.%6N')
 row={name:name,url:post.url,post_id:'424242',input_url:'https://x.com/jack/status/424242',body:body,status:200,path:fetcher.instance_variable_get(:@oracle_path),result:result}
 post.destroy!
 row
end
[['http404',404,''],['http502',502,''],['not found',200,{code:404,tweet:nil}.to_json],['json error',200,'{'],['array',200,'[]'],['depth',200,'['*101+']'*101],['no tweet',200,'{"tweet":false}']].each do |name,status,body|
 post=Twitter::Post.create!(post_id:'424242',url:'https://x.com/jack/status/424242',text:'old text',author_name:'old author',media:[{'type'=>'photo'}])
 f=Twitter::PostFetcher.new(post)
 f.define_singleton_method(:get){|path| @oracle_path=path;[Net::HTTPResponse::CODE_TO_OBJ[status.to_s].new('1.1',status.to_s,''),body]}
 f.fetch
 result=post.reload.attributes.slice(*fields);result['posted_at']=nil
 rows << {name:name,input_url:'https://x.com/jack/status/424242',post_id:'424242',status:status,body:body,path:f.instance_variable_get(:@oracle_path),initial:{text:'old text',author_name:'old author',media:[{type:'photo'}]},result:result}
 post.destroy!
end
[['handleless','https://x.com/i/status/424243','424243'],['empty link',nil,'424244'],['big id','https://x.com/u/status/9999999999999999999999999','9999999999999999999999999']].each do |name,url,id|
 post=Twitter::Post.create!(post_id:id,url:url)
 body={tweet:base.merge('id'=>id)}.to_json
 f=Twitter::PostFetcher.new(post); f.define_singleton_method(:get){|path|@oracle_path=path;[Net::HTTPOK.new('1.1','200','OK'),body]};f.fetch
 result=post.reload.attributes.slice(*fields);result['posted_at']=post.posted_at&.utc&.strftime('%Y-%m-%d %H:%M:%S.%6N')
 rows << {name:name,input_url:url,post_id:id,status:200,body:body,path:f.instance_variable_get(:@oracle_path),result:result};post.destroy!
end
post=Twitter::Post.create!(post_id:'424242',url:'https://x.com/jack/status/424242'); f=Twitter::PostFetcher.new(post);f.define_singleton_method(:get){|_|raise Net::ReadTimeout};f.fetch
rows << {name:'read timeout',input_url:post.url,post_id:post.post_id,path:'/jack/status/424242',transport_error:'ReadTimeout',result:post.reload.attributes.slice(*fields)};post.destroy!
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:'d7c7de92',cases:rows})+"\n")
puts "WS15e X fetch Rails oracle: #{rows.size} persisted cases"
