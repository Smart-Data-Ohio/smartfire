# The real MeetLinkJob/EntrySync/Google::Client pipeline; reject all unlisted network calls.
require 'json'
require 'net/http'
user=User.find(127326141); room=Room.find(699448326); Current.user=user
ActiveJob::Base.queue_adapter=:test
FIXTURE_TOKEN='fixture-calendar-access'
class OlderCalendarHTTP
 def initialize(host) = (@host=host)
 def put(path,body,headers) = request('PUT',path,body,headers)
 def patch(path,body,headers) = request('PATCH',path,body,headers)
 def request(method,path,body,headers)
  routes=Thread.current.fetch(:calendar_routes); route=routes.shift
  raise 'unlisted Google request' unless route && @host=='www.googleapis.com' && method==route[:method] && path==route[:path]
  raise 'wrong credential' unless headers['Authorization']=="Bearer #{FIXTURE_TOKEN}"
  Thread.current[:calendar_calls] << {method:method,path:path,body:JSON.parse(body)}
  response=Net::HTTPResponse::CODE_TO_OBJ.fetch(route[:status].to_s).new('1.1',route[:status].to_s,'fixture')
  response.instance_variable_set(:@read,true); response.body=route[:body].to_json
  response
 end
end
module OlderCalendarNetwork
 def start(host,port,**options)
  raise 'network prohibited' unless host=='www.googleapis.com' && port==443 && options[:use_ssl]
  yield OlderCalendarHTTP.new(host)
 end
end
Net::HTTP.singleton_class.prepend(OlderCalendarNetwork)
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:stream,html:html} }
groups=[]
[4,16].each do |size|
 thread=ChannelThread.create!(room:room,creator:user,name:"older-calendar-job-#{size}")
 event=Event.create!(room:room,organizer:user,title:'Calendar job <&>',starts_at:1.day.from_now,time_zone:'UTC',meet_link_requested:true)
 EventReference.where(event:event).delete_all
 messages=size.times.map do |i|
  m=room.messages.create!(creator:user,thread:i.odd? ? thread : nil,markdown_source:'Older calendar job',client_message_id:"older-calendar-job-#{size}-#{i}")
  m.update_columns(created_at:1.day.ago,updated_at:1.day.ago);EventReference.create!(message:m,event:event);m
 end
 fillers=[nil,thread].flat_map { |t| Message::PAGE_SIZE.times.map { |i| room.messages.create!(creator:user,thread:t,markdown_source:'Newer filler',client_message_id:"older-calendar-job-#{size}-filler-#{t&.id || 'root'}-#{i}") } }
 entry=EventCalendarEntry.create!(event:event,user:user,google_event_id:"fixture-google-copy-#{size}",synced_at:Time.current)
 ids=(messages+fillers).map(&:id).join(',')
 selects={'events'=>"id=#{event.id}",'event_attendances'=>"event_id=#{event.id}",'event_calendar_entries'=>"id=#{entry.id}",'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'event_references'=>"message_id IN (#{ids})"}
 rows=selects.to_h { |t,w| [t,ActiveRecord::Base.connection.select_all("SELECT * FROM #{t} WHERE #{w} ORDER BY id").to_a] }
 streams=["#{room.to_gid_param}:messages","#{thread.to_gid_param}:messages"]
 cases=[]
 %w[success pending forbidden no_account cancelled existing_link disconnected deleted].each do |name|
  event=Event.find_by(id:rows['events'].first['id'])
  event.update_columns(meet_link:nil,cancelled_at:nil)
  account=GoogleAccount.find_or_initialize_by(user:user)
  account.update!(email:'fixture@calendar.test',access_token:FIXTURE_TOKEN,refresh_token:'fixture-calendar-refresh',access_token_expires_at:1.hour.from_now,scopes:Google::Client::CALENDAR_SCOPE,disconnected_reason:nil)
  entry.reload.update_columns(last_error:nil)
  case name
  when 'no_account' then account.delete
  when 'cancelled' then event.update_columns(cancelled_at:Time.current)
  when 'existing_link' then event.update_columns(meet_link:'https://meet.example.test/existing')
  when 'disconnected' then account.update_columns(disconnected_reason:'fixture-disconnected')
  when 'deleted' then event.destroy!
  end
  routes = %w[success pending forbidden].include?(name) ? [
   {method:'PUT',path:"/calendar/v3/calendars/primary/events/#{entry.google_event_id}",status:200,body:{id:entry.google_event_id}},
   {method:'PATCH',path:"/calendar/v3/calendars/primary/events/#{entry.google_event_id}?conferenceDataVersion=1",status:name=='forbidden' ? 403 : 200,body:name=='success' ? {hangoutLink:'https://meet.example.test/job'} : {conferenceData:{createRequest:{status:'pending'}}}}
  ] : []
  Thread.current[:calendar_routes]=routes.map(&:dup);Thread.current[:calendar_calls]=[]
  frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  reads=[];observer=->(*args){p=args.last;reads << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i)}
  ActiveSupport::Notifications.subscribed(observer,'sql.active_record') { Calendar::MeetLinkJob.perform_now(rows['events'].first['id']) }
  raise 'unused Google routes' unless Thread.current[:calendar_routes].empty?
  cases << {name:name,routes:routes,calls:Thread.current[:calendar_calls],retry:ActiveJob::Base.queue_adapter.enqueued_jobs.any? { |j| j[:job]==Calendar::MeetLinkJob },meet_link:Event.find_by(id:rows['events'].first['id'])&.meet_link,reads:reads.size,frames:frames.select { |f| streams.include?(f[:stream]) }.dup}
 end
 groups << {size:size,event_id:rows['events'].first['id'],entry_id:entry.id,thread_id:thread.id,old_ids:messages.map(&:id),rows:rows,cases:cases}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:groups)+"\n")
puts "WS8bm2 older-calendar jobs Rails: #{groups.sum { |g| g[:cases].size }} real jobs; #{groups.sum { |g| g[:cases].sum { |c| c[:frames].size } }} exact frames; 2 pending retries; 10 guarded no-ops; no external network"
