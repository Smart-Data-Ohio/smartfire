require 'json'
user=User.find(127326141);room=Room.find(699448326);Current.user=user
ActiveJob::Base.queue_adapter=:test
conn=ActiveRecord::Base.connection
frames=[];ActionCable.server.define_singleton_method(:broadcast){|stream,html,**|frames << {stream:stream,html:html}}
cases=[]
%w[generic linkedin].each do |kind|
 %w[removed suppressed savepoint_then_removed].each do |mode|
  path="#{kind}-#{mode}";base=kind=='linkedin' ? "https://www.linkedin.com/feed/update/urn:li:activity:#{1_000_000+cases.size*2}" : "https://embed.example.test/#{path}"
  primary=LinkEmbed.create!(normalized_url:base,title:'Before parent',expires_at:1.day.from_now)
  sibling=LinkEmbed.create!(normalized_url:kind=='linkedin' ? "https://www.linkedin.com/feed/update/urn:li:activity:#{1_000_001+cases.size*2}" : "#{base}-sibling",title:'Before sibling',expires_at:1.day.ago,fetch_requested_at:11.minutes.ago)
  message=room.messages.create!(creator:user,markdown_source:'Before message',client_message_id:"ws8-final-state-#{path}")
  [primary,sibling].each_with_index{|e,i|LinkEmbedReference.create!(message:message,link_embed:e,position:i,url:e.normalized_url)}
  select={'link_embeds'=>"id IN (#{primary.id},#{sibling.id})",'messages'=>"id=#{message.id}",'action_text_rich_texts'=>"record_type='Message' AND record_id=#{message.id}",'link_embed_references'=>"message_id=#{message.id}"}
  rows=select.to_h{|t,w|[t,conn.select_all("SELECT * FROM #{t} WHERE #{w} ORDER BY id").to_a]}
  frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  LinkEmbed.transaction do
   if mode=='savepoint_then_removed'
    LinkEmbed.transaction(requires_new:true){primary.update!(title:'Rolled-back intermediate');raise ActiveRecord::Rollback};primary.reload
   end
   primary.update!(title:'After parent')
   if mode=='suppressed'
    message.update_columns(markdown_source:'Newer message state',embeds_suppressed:true)
   else
    message.link_embed_references.delete_all;message.update_columns(markdown_source:'Newer message state')
   end
  end
  pending=ActiveJob::Base.queue_adapter.enqueued_jobs.select{|j|j[:job]==LinkEmbed::FetchJob}.map{|j|j[:args][0]['_aj_globalid'].split('/').last.to_i}
  state=conn.select_one("SELECT * FROM link_embeds WHERE id=#{sibling.id}")
  cases << {kind:kind,mode:mode,rows:rows,primary_id:primary.id,sibling_id:sibling.id,message_id:message.id,pending:pending,sibling:state,message:conn.select_one("SELECT * FROM messages WHERE id=#{message.id}"),frames:frames.select{|f|f[:stream]=="#{room.to_gid_param}:messages"}.dup}
  puts "WS8bm2 Rails final-state siblings #{kind}/#{mode}: #{pending.size} sibling jobs; #{cases.last[:frames].size} frames; sibling claim=#{state['fetch_requested_at']}"
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(cases)+"\n")

puts "WS8bm2 final-state siblings Rails: #{cases.size} final-state transactions; zero sibling jobs; unchanged claims; exact frames"
