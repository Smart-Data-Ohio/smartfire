# Deterministic deletion during GET and actual durable-job write failure, without external network.
require 'json'
require 'net/http'
require 'restricted_http/private_network_guard'
require_relative 'oracle-database'
ActiveJob::Base.queue_adapter=:test
RestrictedHTTP::PrivateNetworkGuard.define_singleton_method(:resolve) { |host| raise 'unlisted DNS' unless ['embed.example.test','www.linkedin.com'].include?(host); '93.184.216.34' }
class EmbedFailureHTTP
 attr_accessor :max_retries
 def request(request, *_args, &block)
  data=Thread.current.fetch(:embed_failure)
  raise 'unlisted embed failure request' unless request.method=='GET' && request.path==data[:path] && request['Cookie'].nil? && request['Authorization'].nil?
  data[:calls] << {method:request.method,path:request.path}
  LinkEmbed.find(data[:id]).destroy! if data[:name]=='deleted_during_fetch'
  response=Net::HTTPOK.new('1.1','200','OK');response.instance_variable_set(:@read,true)
  response['Content-Type']='text/html';html='<meta property="og:title" content="After deterministic fetch">';response.body=html
  response.define_singleton_method(:read_body){|*_,&yield_body|yield_body ? yield_body.call(html) : html}
  block ? block.call(response) : response
 end
end
module EmbedFailureNetwork
 def start(host,port,**options)
  raise 'external network prohibited' unless host==Thread.current.fetch(:embed_failure)[:host] && port==443 && options[:use_ssl]
  yield EmbedFailureHTTP.new
 end
end
Net::HTTP.singleton_class.prepend(EmbedFailureNetwork)
frames=[];ActionCable.server.define_singleton_method(:broadcast){|stream,html,**|frames << {stream:,html:}}
user=User.find(127326141);room=Room.find(699448326);Current.user=user;groups=[]
%w[generic linkedin].each do |kind|;[4,16].each do |size|
 host=kind=='linkedin' ? 'www.linkedin.com' : 'embed.example.test';path=kind=='linkedin' ? "/feed/update/urn:li:activity:#{99000+size}" : "/failure-#{size}"
 thread=ChannelThread.create!(room:,creator:user,name:"embed-failure-#{kind}-#{size}")
 parent=LinkEmbed.create!(normalized_url:"https://#{host}#{path}",title:'Before job',expires_at:1.day.from_now)
 sibling=LinkEmbed.create!(normalized_url:"https://#{host}#{path}-sibling",title:'Stale sibling',expires_at:1.day.ago)
 messages=size.times.map do |i|
  m=room.messages.create!(creator:user,thread:i.odd? ? thread : nil,markdown_source:'Older failure reference',client_message_id:"embed-failure-#{kind}-#{size}-#{i}")
  m.update_columns(created_at:1.day.ago,updated_at:1.day.ago)
  [parent,sibling].each_with_index{|e,p|LinkEmbedReference.create!(message:m,link_embed:e,position:p,url:e.normalized_url)};m
 end
 fillers=[nil,thread].flat_map{|t|Message::PAGE_SIZE.times.map{|i|room.messages.create!(creator:user,thread:t,markdown_source:'Newer filler',client_message_id:"embed-failure-#{kind}-#{size}-filler-#{t&.id||'root'}-#{i}")}}
 sibling.update_columns(fetch_requested_at:11.minutes.ago)
 ids=(messages+fillers).map(&:id).join(',');selects={'link_embeds'=>"id IN (#{parent.id},#{sibling.id})",'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'link_embed_references'=>"message_id IN (#{ids})"}
 rows=selects.to_h{|t,w|[t,ActiveRecord::Base.connection.select_all("SELECT * FROM #{t} WHERE #{w} ORDER BY id").to_a]}
 reset=MessagingOracleDatabase.scenarios(ARGV.fetch(0));cases=[]
 %w[deleted_before_job deleted_during_fetch write_failed].each do |name|
  reset.call do
   p=LinkEmbed.find(parent.id);s=LinkEmbed.find(sibling.id);before=ActiveRecord::Base.connection.select_all("SELECT * FROM link_embeds WHERE id IN (#{p.id},#{s.id}) ORDER BY id").to_a
   Thread.current[:embed_failure]={id:p.id,host:,path:,name:,calls:[]};frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
   LinkEmbed::FetchJob.perform_later(p);job=ActiveJob::Base.queue_adapter.enqueued_jobs.shift
   p.destroy! if name=='deleted_before_job'
   ActiveRecord::Base.connection.execute("CREATE TRIGGER ws8_failure BEFORE UPDATE ON link_embeds WHEN OLD.id=#{p.id} BEGIN SELECT RAISE(ABORT,'fixture embed writer failure'); END") if name=='write_failed'
   error=nil
   begin
    ActiveJob::Base.execute(job.except(:job,:args,:queue,:priority))
   rescue StandardError=>e
    error=e.class.name
   end
   after=ActiveRecord::Base.connection.select_all("SELECT * FROM link_embeds WHERE id IN (#{p.id},#{s.id}) ORDER BY id").to_a
   raise 'failed writer did not roll back' if name=='write_failed' && after!=before
   streams=["#{room.to_gid_param}:messages","#{thread.to_gid_param}:messages"]
   frames.select! {|frame| streams.include?(frame[:stream])}
   raise "silent job published #{kind}/#{size}/#{name}" unless frames.empty?
   raise 'silent job enqueued sibling' unless ActiveJob::Base.queue_adapter.enqueued_jobs.empty?
   cases << {name:,outcome:error ? 'failed' : 'completed',error:,calls:Thread.current[:embed_failure][:calls],after:,frames:frames.dup,pending:[]}
  end
 end
 groups << {kind:,size:,embed_id:parent.id,sibling_id:sibling.id,host:,path:,thread_id:thread.id,rows:,cases:}
end;end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',groups:) + "\n")
puts "WS8bm2 older-embed failure Rails: #{groups.sum{|g|g[:cases].size}} actual queued jobs; 4 deterministic deletions during GET; 4 discarded missing records; 4 failed writes with unchanged siblings, no frames or child jobs"
