# Old reference windows through actual InboundSyncJob and queued SyncEntryJob consumers.
require 'json'
require 'net/http'
require_relative 'oracle-database'
ActiveJob::Base.queue_adapter=:test
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
FIXTURE_TOKEN='fixture-calendar-access'
class CalendarExecutionHTTP
 def initialize(host)=(@host=host)
 def get(path,headers)=request('GET',path,nil,headers)
 def delete(path,headers)=request('DELETE',path,nil,headers)
 def post(path,body,headers)=request('POST',path,body,headers)
 def put(path,body,headers)=request('PUT',path,body,headers)
 def request(method,path,body,headers)
  route=Thread.current.fetch(:calendar_routes).shift
  raise "unlisted Google request #{method} #{path}; expected #{route.inspect}" unless route && @host=='www.googleapis.com' && method==route[:method] && path==route[:path]
  raise 'wrong fixture credential' unless headers['Authorization']==['Bearer',FIXTURE_TOKEN].join(' ')
  Thread.current[:calendar_calls] << {method:,path:,body:body && JSON.parse(body)}
  response=Net::HTTPResponse::CODE_TO_OBJ.fetch(route[:status].to_s).new('1.1',route[:status].to_s,'fixture')
  response.instance_variable_set(:@read,true);response.body=route[:body].to_json;response
 end
end
module CalendarExecutionNetwork
 def start(host,port,**options)
  raise 'network prohibited' unless host=='www.googleapis.com' && port==443 && options[:use_ssl]
  yield CalendarExecutionHTTP.new(host)
 end
end
Net::HTTP.singleton_class.prepend(CalendarExecutionNetwork)
frames=[];ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:,html:} }
user=User.find(127326141);room=Room.find(699448326);Current.user=user;groups=[]
Icons.custom_icons;Icons.instance_variable_set(:@custom_cache_at,Float::INFINITY)
[4,16].each do |size|
 EventCalendarEntry.delete_all
 thread=ChannelThread.create!(room:,creator:user,name:"older-calendar-execution-#{size}")
 event=Event.create!(room:,organizer:user,title:'Calendar execution <&>',starts_at:1.day.from_now,time_zone:'UTC')
 EventReference.where(event:).delete_all
 messages=size.times.map do |i|
  m=room.messages.create!(creator:user,thread:i.odd? ? thread : nil,markdown_source:'Older calendar execution',client_message_id:"calendar-execution-#{size}-#{i}")
  m.update_columns(created_at:1.day.ago,updated_at:1.day.ago);EventReference.create!(message:m,event:);m
 end
 fillers=[nil,thread].flat_map { |t| Message::PAGE_SIZE.times.map { |i|room.messages.create!(creator:user,thread:t,markdown_source:'Newer filler',client_message_id:"calendar-execution-#{size}-filler-#{t&.id || 'root'}-#{i}")} }
 entry=EventCalendarEntry.create!(event:,user:,google_event_id:"fixture-google-execution-#{size}",synced_at:Time.current)
 account=GoogleAccount.find_or_initialize_by(user:);account.update!(email:'fixture@calendar.test',access_token:FIXTURE_TOKEN,refresh_token:'fixture-calendar-refresh',access_token_expires_at:1.hour.from_now,scopes:Google::Client::CALENDAR_SCOPE,disconnected_reason:nil)
 ids=(messages+fillers).map(&:id).join(',');selects={'events'=>"id=#{event.id}",'event_attendances'=>"event_id=#{event.id}",'event_calendar_entries'=>"id=#{entry.id}",'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'event_references'=>"message_id IN (#{ids})"}
 rows=selects.to_h{|t,w|[t,ActiveRecord::Base.connection.select_all("SELECT * FROM #{t} WHERE #{w} ORDER BY id").to_a]}
 reset=MessagingOracleDatabase.scenarios(ARGV.fetch(0));cases=[]
 %w[inbound_confirmed inbound_cancelled inbound_deleted inbound_forbidden inbound_unavailable inbound_no_account inbound_disconnected inbound_departed inbound_local_declined sync_update sync_conflict sync_unavailable sync_deleted sync_write_failed sync_error_write_failed sync_delete_failed].each do |name|
  reset.call do
   e=Event.find(event.id);a=GoogleAccount.find(account.id);copy=EventCalendarEntry.find(entry.id)
   a.delete if name=='inbound_no_account'
   a.update_columns(disconnected_reason:'fixture-disconnected') if name=='inbound_disconnected'
   room.memberships.where(user:).delete_all if name=='inbound_departed'
   EventAttendance.where(event:e,user:).update_all(response:'declined') if name=='inbound_local_declined'
   copy.update_columns(synced_at:nil,last_error:'fixture prior calendar failure') if %w[sync_conflict sync_write_failed].include?(name)
   EventAttendance.where(event:e,user:).update_all(response:'declined') if name=='sync_delete_failed'
   e.destroy! if name=='sync_deleted'
   path="/calendar/v3/calendars/primary/events/#{copy.google_event_id}"
   routes=if name.start_with?('inbound')
    %w[inbound_no_account inbound_disconnected inbound_departed].include?(name) ? [] : [{method:'GET',path:,status:name=='inbound_deleted' ? 404 : name=='inbound_forbidden' ? 403 : name=='inbound_unavailable' ? 503 : 200,body:{status:name=='inbound_cancelled' ? 'cancelled' : 'confirmed'}}]
   else
    name=='sync_deleted' ? [] : %w[sync_conflict sync_write_failed].include?(name) ? [{method:'POST',path:'/calendar/v3/calendars/primary/events',status:409,body:{}},{method:'PUT',path:,status:200,body:{id:copy.google_event_id}}] : [{method:'PUT',path:,status:%w[sync_unavailable sync_error_write_failed].include?(name) ? 503 : 200,body:{id:copy.google_event_id}}]
   end
   routes=[{method:'DELETE',path:,status:204,body:{}}] if name=='sync_delete_failed'
   fatal=%w[sync_write_failed sync_error_write_failed sync_delete_failed].include?(name)
   if fatal
    operation=name=='sync_delete_failed' ? 'DELETE' : 'UPDATE'
    ActiveRecord::Base.connection.execute("CREATE TRIGGER ws8_calendar_failure BEFORE #{operation} ON event_calendar_entries WHEN OLD.id=#{copy.id} BEGIN SELECT RAISE(ABORT,'fixture calendar writer failure'); END")
   end
   routes << {method:'DELETE',path:,status:204,body:{}} if %w[inbound_cancelled inbound_deleted].include?(name)
   Thread.current[:calendar_routes]=routes.map(&:dup);Thread.current[:calendar_calls]=[];frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
   error=nil
   reads=[];observer=->(*args){p=args.last;reads << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i)}
   ActiveSupport::Notifications.subscribed(observer,'sql.active_record') do
    begin
     name.start_with?('inbound') ? Calendar::InboundSyncJob.perform_now(user.id) : Calendar::SyncEntryJob.perform_now(e.id,user.id)
    rescue StandardError=>ex
     raise unless fatal && ex.message.include?('fixture calendar writer failure')
     error=ex.class.name
    end
    if %w[inbound_cancelled inbound_deleted].include?(name)
     queued=ActiveJob::Base.queue_adapter.enqueued_jobs.select{|j|j[:job]==Calendar::SyncEntryJob};raise 'missing queued child' unless queued.size==1
     queued.each {|j|Calendar::SyncEntryJob.perform_now(*j[:args])}
    end
   end
   raise 'missing Calendar writer failure' if fatal && !error
   raise 'unused routes' unless Thread.current[:calendar_routes].empty?
   retrying=ActiveJob::Base.queue_adapter.enqueued_jobs.any?{|j|j[:job].name==(name.start_with?('inbound') ? 'Calendar::InboundSyncJob' : 'Calendar::SyncEntryJob') && j[:at]}
   state=EventAttendance.find_by(event_id:event.id,user_id:user.id)&.response
   cases << {name:,routes:,calls:Thread.current[:calendar_calls].dup,retry:retrying,response:state,entry_present:EventCalendarEntry.exists?(id:entry.id),entry_row:ActiveRecord::Base.connection.select_one("SELECT * FROM event_calendar_entries WHERE id=#{entry.id}"),error:,reads:reads.size,frames:frames.select{|f|["#{room.to_gid_param}:messages","#{thread.to_gid_param}:messages"].include?(f[:stream])}.dup}
  end
 end
 groups << {size:,event_id:event.id,entry_id:entry.id,thread_id:thread.id,old_ids:messages.map(&:id),rows:,cases:}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',groups:)+"\n")
puts "WS8bm2 Calendar execution Rails: #{groups.sum{|g|g[:cases].size}} parent jobs; 4 queued SyncEntry children executed; #{groups.sum{|g|g[:cases].sum{|c|c[:frames].size}}} exact old-window frames"
puts "WS8bm2 Calendar execution Rails reads: #{groups.map{|g|[g[:size],g[:cases].map{|c|c[:reads]}.join('/')].join(':')}.join(', ')}"
