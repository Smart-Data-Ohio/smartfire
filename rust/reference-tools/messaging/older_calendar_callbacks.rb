# Event after_update_commit snapshots and old root/reply windows, with real Rails frames.
require 'json'
require 'digest'
user = User.find(127326141); room = Room.find(699448326); Current.user = user
ActiveJob::Base.queue_adapter = :test
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:stream,html:html} }
groups = []
[false,true].each do |populated|
 [4,16].each do |size|
  prefix = "older-calendar-#{populated}-#{size}"
  thread = ChannelThread.create!(room:room,creator:user,name:prefix)
  event = Event.create!(room:room,organizer:user,title:'Old event',starts_at:1.day.from_now,ends_at:1.day.from_now+1.hour,time_zone:'Eastern Time (US & Canada)')
  sibling = Event.create!(room:room,organizer:user,title:'Sibling <&> event',starts_at:2.days.from_now,time_zone:'UTC') if populated
  # Announcements are outside this isolated consumer fixture. Keep only explicitly selected references.
  EventReference.where(event_id:[event.id,sibling&.id].compact).delete_all
  messages = size.times.map do |i|
   message = room.messages.create!(creator:user,thread:i.odd? ? thread : nil,markdown_source:'Older event reference',client_message_id:"#{prefix}-#{i}")
   message.update_columns(created_at:1.day.ago,updated_at:1.day.ago,embeds_suppressed:i==size-1)
   [event,sibling].compact.each { |e| EventReference.create!(message:message,event:e) }
   message
  end
  fillers = [nil,thread].flat_map { |t| Message::PAGE_SIZE.times.map { |i| room.messages.create!(creator:user,thread:t,markdown_source:'Newer filler',client_message_id:"#{prefix}-filler-#{t&.id || 'root'}-#{i}") } }
  ids = (messages+fillers).map(&:id).join(',')
  selects={'events'=>"id IN (#{[event.id,sibling&.id].compact.join(',')})",'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'event_references'=>"message_id IN (#{ids})"}
  rows = selects.to_h { |table,where| [table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a] }
  streams = ["#{room.to_gid_param}:messages","#{thread.to_gid_param}:messages"]
  raise 'root window contains old references' unless (room.root_messages.last_page.pluck(:id)&messages.map(&:id)).empty?
  raise 'reply window contains old references' unless (thread.messages.last_page.pluck(:id)&messages.map(&:id)).empty?
  steps = []
  %w[title twice meet cancel].each do |step|
   frames.clear; queries=[]
   observer = ->(*args) { p=args.last;queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
   ActiveSupport::Notifications.subscribed(observer,'sql.active_record') do
    case step
    when 'title' then event.update!(title:'Updated <&> title')
    when 'twice' then Event.transaction { event.update!(title:'Intermediate'); event.update!(title:'Final <&> title') }
    when 'meet' then event.update!(meet_link:'https://meet.example.test/fixture')
    when 'cancel' then event.cancel!(actor:user)
    end
   end
   steps << {name:step,reads:queries.size,frames:frames.select { |f| streams.include?(f[:stream]) }.dup}
  end
  frames.clear
  Event.transaction { event.update!(title:'Rolled back secret'); raise ActiveRecord::Rollback }
  raise 'rollback broadcast' unless frames.empty?
  raise 'rollback leaked' unless event.reload.title == 'Final <&> title'
  # Callback association lookup happens after commit, including reference removal in the transaction.
  frames.clear
  Event.transaction { event.update!(title:'Reference removed'); messages.first.event_references.where(event:event).delete_all }
  steps << {name:'removed_reference',frames:frames.select { |f| streams.include?(f[:stream]) }.dup}
  groups << {size:size,populated:populated,event_id:event.id,thread_id:thread.id,rows:rows,old_ids:messages.map(&:id),steps:steps}
 end
end
# Boundaries exercise Event associations and organizer preloads across find_each batches.
boundaries=[]
[999,1000,1001].each do |count|
 ActionCable.server.define_singleton_method(:broadcast) { |*_, **| }
 event=Event.create!(room:room,organizer:user,title:'Before',starts_at:1.day.from_now,time_zone:'UTC')
 EventReference.where(event:event).delete_all
 base=Message.maximum(:id)
 conn=ActiveRecord::Base.connection
 conn.execute("WITH RECURSIVE ids(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM ids WHERE n<#{count}) INSERT INTO messages (id,room_id,creator_id,client_message_id,embeds_suppressed,created_at,updated_at) SELECT #{base}+n,#{room.id},#{user.id},'bounded-event-'||n,1,'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM ids")
 conn.execute("INSERT INTO event_references (message_id,event_id,created_at,updated_at) SELECT id,#{event.id},'2026-03-01 16:00:00','2026-03-01 16:00:00' FROM messages WHERE id>#{base}")
 rows={'events'=>conn.select_all("SELECT * FROM events WHERE id=#{event.id}").to_a}
 digest=Digest::SHA256.new; n=0; first=nil; last=nil
 ActionCable.server.define_singleton_method(:broadcast) do |stream,html,**|
  raise 'wrong stream' unless stream=="#{room.to_gid_param}:messages"
  n+=1; raise 'wrong order' unless html.include?("bounded-event-#{n}\"")
  first ||= html; last=html; digest << html << "\n"
 end
 event.update!(title:'After <&>')
 raise 'missing frames' unless n==count
 boundaries << {count:count,base_id:base,event_id:event.id,rows:rows,sha256:digest.hexdigest,first:first,last:last}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',groups:groups,boundaries:boundaries)+"\n")
puts "WS8bm2 older-calendar Rails: #{groups.size} groups; #{groups.sum { |g| g[:steps].sum { |s| s[:frames].size } }} exact frames; 4 silent rollbacks; #{boundaries.sum { |b| b[:count] }} ordered boundary frames"
puts "WS8bm2 older-calendar Rails reads: #{groups.map { |g| [g[:populated],g[:size],g[:steps].first[:reads]].join(':') }.join(', ')}"
