# Actual queued stale children after each network parent; both providers and old windows.
require 'json'
require 'net/http'
require 'restricted_http/private_network_guard'
user=User.find(127326141);room=Room.find(699448326);Current.user=user
ActiveJob::Base.queue_adapter=:test
RestrictedHTTP::PrivateNetworkGuard.define_singleton_method(:resolve) do |host|
 raise RestrictedHTTP::Violation,'fixture private host' if host=='127.0.0.1'
 raise 'unlisted DNS' unless ['embed.example.test','www.linkedin.com'].include?(host)
 '93.184.216.34'
end
class OlderEmbedHTTP
 attr_accessor :max_retries
 def initialize(host) = (@host=host)
 def request(request)
  raise 'unexpected credential' if request['Cookie'] || request['Authorization']
  route=Thread.current.fetch(:embed_routes).find { |r| r[:host]==@host && r[:method]==request.method && r[:path]==request.path }
  raise 'unlisted HTTP request' unless route
  Thread.current[:embed_calls] << {host:@host,method:request.method,path:request.path}
  response=Net::HTTPResponse::CODE_TO_OBJ.fetch(route[:status].to_s).new('1.1',route[:status].to_s,'fixture')
  route.fetch(:headers,{}).each { |k,v| response[k]=v }
  response.instance_variable_set(:@read,true);response.body=route[:body].to_s
  response.define_singleton_method(:read_body) { |*_,&block| block ? block.call(route[:body].to_s) : route[:body].to_s }
  block_given? ? yield(response) : response
 end
end
module OlderEmbedNetwork
 def start(host,port,**options)
  raise 'network prohibited' unless ['embed.example.test','www.linkedin.com'].include?(host) && port==443 && options[:use_ssl] && options[:ipaddr]=='93.184.216.34'
  yield OlderEmbedHTTP.new(host)
 end
end
Net::HTTP.singleton_class.prepend(OlderEmbedNetwork)
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:stream,html:html} }
groups=[]
['generic','linkedin'].each do |kind|
 [4,16].each do |size|
  host=kind=='linkedin' ? 'www.linkedin.com' : 'embed.example.test'
  path=kind=='linkedin' ? "/feed/update/urn:li:activity:#{90000+size}" : "/job-#{size}"
  prefix="older-embed-job-#{kind}-#{size}"
  thread=ChannelThread.create!(room:room,creator:user,name:prefix)
  primary=LinkEmbed.create!(normalized_url:"https://#{host}#{path}",title:'Old preview',description:'Old description',expires_at:1.day.from_now)
  sibling=LinkEmbed.create!(normalized_url:kind=='linkedin' ? "https://#{host}/feed/update/urn:li:activity:#{91000+size}" : "https://#{host}#{path}-sibling",title:'Stale sibling',expires_at:1.day.ago)
  opposite=LinkEmbed.create!(normalized_url:kind=='linkedin' ? "https://embed.example.test/opposite-#{size}" : "https://www.linkedin.com/feed/update/urn:li:activity:#{80000+size}",title:'Opposite provider',expires_at:1.day.ago)
  suppressed=LinkEmbed.create!(normalized_url:kind=='linkedin' ? "https://#{host}/feed/update/urn:li:activity:#{92000+size}" : "https://#{host}#{path}-suppressed",title:'Suppressed only',expires_at:1.day.ago)
  messages=size.times.map do |i|
   m=room.messages.create!(creator:user,thread:i.odd? ? thread : nil,markdown_source:'Older embed job',client_message_id:"#{prefix}-#{i}")
   m.update_columns(created_at:1.day.ago,updated_at:1.day.ago,embeds_suppressed:i==size-1)
   [primary,sibling,opposite].each_with_index { |e,p| LinkEmbedReference.create!(message:m,link_embed:e,position:p,url:e.normalized_url) }
   LinkEmbedReference.create!(message:m,link_embed:suppressed,position:3,url:suppressed.normalized_url) if i==size-1
   m
  end
  fillers=[nil,thread].flat_map { |t| Message::PAGE_SIZE.times.map { |i| room.messages.create!(creator:user,thread:t,markdown_source:'Newer filler',client_message_id:"#{prefix}-filler-#{t&.id || 'root'}-#{i}") } }
  [primary,sibling,opposite,suppressed].each { |e| e.update_columns(fetch_requested_at:11.minutes.ago) }
  ids=(messages+fillers).map(&:id).join(',');eids=[primary,sibling,opposite,suppressed].map(&:id).join(',')
  selects={'link_embeds'=>"id IN (#{eids})",'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'link_embed_references'=>"message_id IN (#{ids})"}
  rows=selects.to_h { |t,w| [t,ActiveRecord::Base.connection.select_all("SELECT * FROM #{t} WHERE #{w} ORDER BY id").to_a] }
  streams=["#{room.to_gid_param}:messages","#{thread.to_gid_param}:messages"]
  jobs=[]
  %w[success login http_error redirect private_image].each do |name|
   sibling.update_columns(fetch_requested_at:11.minutes.ago,expires_at:1.day.ago)
   image="https://#{host}/fixture.png"
   html="<meta property='og:title' content='Network &lt;b&gt;title&lt;/b&gt; &amp;'><meta name='description' content='Fetched description'><meta property='og:image' content='#{image}'>"
   body = name=='login' ? '<meta property="og:site_name" content="Login">' : name=='private_image' ? '<title>Private image</title><meta property="og:image" content="https://127.0.0.1/private.png">' : html
   routes=[{host:host,method:'GET',path:path,status:name=='http_error' ? 502 : name=='redirect' ? 302 : 200,headers:name=='redirect' ? {'Location'=>'/landing'} : {'Content-Type'=>'text/html'},body:body}]
   routes << {host:host,method:'GET',path:'/landing',status:200,headers:{'Content-Type'=>'text/html'},body:html} if name=='redirect'
   routes << {host:host,method:'HEAD',path:'/fixture.png',status:200,headers:{'Content-Type'=>'image/png'},body:''} if ['success','redirect'].include?(name)
   child_path=URI(sibling.normalized_url).request_uri
   routes << {host:host,method:'GET',path:child_path,status:name=='http_error' ? 502 : 200,headers:{'Content-Type'=>'text/html'},body:"<title>Child #{name} &amp; preview</title>"}
   Thread.current[:embed_routes]=routes;Thread.current[:embed_calls]=[]
   frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
   reads=[];observer=->(*args){p=args.last;reads << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i)}
   ActiveSupport::Notifications.subscribed(observer,'sql.active_record') { LinkEmbed::FetchJob.perform_now(primary.reload) }
   pending=ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job]==LinkEmbed::FetchJob }.map { |j| j[:args][0]['_aj_globalid'].split('/').last.to_i }
   raise "sibling job not deduplicated #{kind}/#{size}/#{name}: #{pending.inspect}, expected #{sibling.id}" unless pending==[sibling.id]
   queued=ActiveJob::Base.queue_adapter.enqueued_jobs.find { |j| j[:job]==LinkEmbed::FetchJob }
   raise 'queued child identity changed' unless GlobalID::Locator.locate(queued[:args][0]['_aj_globalid']).id==sibling.id
   ActiveJob::Base.queue_adapter.enqueued_jobs.delete(queued)
   child_reads=[];child_observer=->(*args){p=args.last;child_reads << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i)}
   ActiveSupport::Notifications.subscribed(child_observer,'sql.active_record') {LinkEmbed::FetchJob.perform_now(sibling.reload)}
   child=sibling.reload.attributes.slice('title','description','site_name','image_url','fetch_error','fetched_at','expires_at')
   raise 'child queued another fetch' if ActiveJob::Base.queue_adapter.enqueued_jobs.any? { |j| j[:job]==LinkEmbed::FetchJob }
   state=primary.reload.attributes.slice('title','description','site_name','image_url','fetch_error','fetched_at','expires_at')
   jobs << {name:name,routes:routes,calls:Thread.current[:embed_calls],state:state,pending:pending,reads:reads.size,child_reads:child_reads.size,child:child,frames:frames.select { |f| streams.include?(f[:stream]) }.dup}
  end
  # A stale-sibling claim, queue write and parent save must all roll back together.
  frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  sibling.update_columns(fetch_requested_at:11.minutes.ago)
  before=primary.reload.attributes;claim=sibling.reload.fetch_requested_at
  [false,true].each do |nested|
   LinkEmbed.transaction do
    if nested
     LinkEmbed.transaction(requires_new:true) { primary.update!(title:'Rolled back secret'); raise ActiveRecord::Rollback }
    else
     primary.update!(title:'Rolled back secret'); raise ActiveRecord::Rollback
    end
   end
   raise 'rollback leaked' unless frames.empty? && primary.reload.attributes==before && sibling.reload.fetch_requested_at==claim && ActiveJob::Base.queue_adapter.enqueued_jobs.empty?
  end
  raise 'rollback leaked' unless frames.empty? && primary.reload.attributes==before && sibling.reload.fetch_requested_at==claim && ActiveJob::Base.queue_adapter.enqueued_jobs.empty?
  groups << {kind:kind,size:size,embed_id:primary.id,sibling_id:sibling.id,opposite_id:opposite.id,suppressed_id:suppressed.id,thread_id:thread.id,old_ids:messages.map(&:id),rows:rows,jobs:jobs}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:groups)+"\n")
puts "WS8bm2 older-embed children Rails: 20 real network jobs; #{groups.sum { |g| g[:jobs].sum { |j| j[:frames].size } }} exact frames; 20 queued same-provider children executed; 8 silent outer/savepoint rollbacks"
puts "WS8bm2 older-embed children Rails reads: #{groups.map { |g| [g[:kind],g[:size],g[:jobs].map { |j| "#{j[:reads]}/#{j[:child_reads]}" }.join('/')].join(':') }.join(', ')}"
