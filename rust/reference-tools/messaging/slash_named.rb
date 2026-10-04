# The 35 built-in DispatcherTest cases; actual handlers and committed hooks, no handler stubs.
require 'json'
require 'active_support/testing/time_helpers'
require_relative 'oracle-database'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,9,23,12)
ActiveJob::Base.queue_adapter=:test
frames=[];ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:stream,html:html} }
reset=MessagingOracleDatabase.scenarios(ARGV.fetch(0))
names=JSON.parse("[\"registry holds every shipped command with metadata\", \"command_text? matches slash commands but not escapes or play passthrough\", \"huddle starts a call when configured\", \"huddle errors when unconfigured\", \"event opens the prefilled form url\", \"event without a time prefills the title only\", \"event rejects past times\", \"bare event opens the blank form\", \"poll opens the builder in channels but not threads\", \"remind posts and saves with a reminder\", \"remind rejects unusable input without posting\", \"status sets emoji and text until end of day\", \"status rejects blank arguments\", \"dnd toggles, takes durations, and turns off\", \"dnd rejects garbage durations\", \"ooo sets an end with a note, and off clears it\", \"ooo takes week durations, dates, and datetimes\", \"ooo bare tomorrow and weekdays run to the end of the day\", \"ooo bare dates run to the end of the day\", \"ooo bare month dates roll to next year when this year's passed\", \"ooo day durations stay exact\", \"ooo broadcasts the badge and the notice\", \"ooo off while calendar OOO covers says the calendar still shows it\", \"ooo rejects blank arguments, garbage, past times, and long notes\", \"shrug posts with the shrug\", \"posting commands in a board answer an error without posting\", \"posting commands in a board thread still post\", \"slash posts in threads skip the legacy webhook fanout\", \"slash posts in channels fan out to legacy webhooks\", \"slash posts in threads process attachments once\", \"me posts an action line\", \"me requires an action\", \"play posts through the normal message path\", \"slash posts never start a stream\", \"unknown commands error with the available list\"]")
actions=[nil,nil,['/huddle'],['/huddle'],['/event Launch party friday 5pm'],['/event Launch party'],['/event Retro yesterday','/event Retro 2026-09-01 15:00'],['/event'],['/poll','/poll'],['/remind in 20 minutes review the deploy'],['/remind sometime review the deploy','/remind in 20 minutes'],['/status 🚂 On a train'],['/status'],['/dnd','/dnd','/dnd 2h','/dnd off'],['/dnd eventually'],['/ooo 2h Back soon','/ooo off'],['/ooo 1 week','/ooo friday 5pm Wrapping up','/ooo 2026-10-01 15:00'],['/ooo tomorrow Back soon','/ooo friday','/ooo monday Wrapping up'],['/ooo 2026-10-05','/ooo oct 8 Back soon'],['/ooo sep 1'],['/ooo 3d'],['/ooo tomorrow Back soon'],['/ooo off'],['/ooo','/ooo eventually','/ooo 2026-09-01 15:00',"/ooo tomorrow #{'x'*141}"],['/shrug ship it'],['/shrug ship it'],['/shrug ship it'],['/shrug Hey @[Legacy Note]'],['/shrug Hey @[Legacy Note]'],['/shrug hi'],['/me is reviewing the deploy'],['/me'],['/play tada'],['/shrug ship it'],['/frobnicate']]
cases=[]
names.each_with_index do |name,i|
 reset.call do
  Current.reset;user=User.find(127326141);user.update!(time_zone:'America/New_York');Current.user=user;room=Room.find(654632876);thread=nil;legacy=nil
  config=i==2
  Huddle::REQUIRED_ENVIRONMENT.each { |key| ENV[key]=config ? key.include?('URL') ? key.include?('INTERNAL') ? 'http://livekit:7880' : 'https://fixture-gateway.test' : "fixture-#{key.downcase}" : nil }
  raise 'real huddle configuration mismatch' unless Huddle.configured? ==config
  thread=ChannelThread.create!(room:room,creator:user,name:'Side chat') if [8,27,29].include?(i)
  room=Rooms::Board.create_for({name:'Launch',creator:user},users:[user]) if [25,26].include?(i)
  thread=ChannelThread.create_board_post!(room:room,creator:user,name:'Stuck migration',work_status:'in_progress') if i==26
  if [27,28].include?(i)
   legacy=User.create_bot!(name:'Legacy Note',webhook_url:'https://example.test/legacy-note');legacy.update_columns(bot_token_digest:Digest::SHA256.hexdigest('fixture-legacy-key'));room.memberships.grant_to(legacy)
  end
  if i==22
   user.update!(ooo_calendar_enabled:true,ooo_until:1.day.from_now)
   Calendar::MeetingCache.create!(user:user,fetched_at:Time.current,ooo_intervals:[[5.minutes.ago.iso8601,3.days.from_now.iso8601]])
  end
  initial=user.reload.attributes.slice('time_zone','status','last_active_at','dnd_enabled','dnd_until','ooo_calendar_enabled','ooo_until','ooo_note','custom_status_emoji','custom_status_text','custom_status_expires_at')
  initial['status']=User.statuses.fetch(initial['status'])
  tables=%w[users rooms memberships channel_threads webhooks calendar_meeting_caches messages action_text_rich_texts]
  ids={'users'=>legacy ? "id=#{legacy.id}" : '0','rooms'=>[25,26].include?(i) ? "id=#{room.id}" : '0','memberships'=>[25,26].include?(i) ? "room_id=#{room.id}" : legacy ? "user_id=#{legacy.id}" : '0','channel_threads'=>thread ? "id=#{thread.id}" : '0','webhooks'=>legacy ? "user_id=#{legacy.id}" : '0','calendar_meeting_caches'=>"user_id=#{user.id}",'messages'=>[25,26].include?(i) ? "room_id=#{room.id}" : '0','action_text_rich_texts'=>[25,26].include?(i) ? "record_type='Message' AND record_id IN (SELECT id FROM messages WHERE room_id=#{room.id})" : '0'}
  rows=tables.to_h { |t| [t,ActiveRecord::Base.connection.select_all("SELECT * FROM #{t} WHERE #{ids[t]} ORDER BY id").to_a.map { |r| t=='users' ? r.except('password_digest') : r }] }
  observations=[];frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  if i==0
   observations=SlashCommands::Registry.all.map { |c| {name:c.name,description:c.description,arg_hint:c.arg_hint,takes_arguments:c.takes_arguments,root:c.permission.call(user,room,nil),thread:c.permission.call(user,room,Object.new)} }
  elsif i==1
   observations=['/poll','/me waves','//poll','hello /poll','just text'].map { |text| {text:,recognized:SlashCommands::Dispatcher.command_text?(text)} }
  else
   actions[i].each_with_index do |text,n|
    frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear;calls=0
    tracing=TracePoint.new(:call) { |t| calls+=1 if t.method_id==:process_attachment && t.self.is_a?(Message) }
    selected_thread=i==8 && n==0 ? nil : thread
    count_before=Message.count
    result=tracing.enable { SlashCommands::Dispatcher.dispatch(user:user,room:room,thread:selected_thread,text:) }
    count_after=Message.count
    posted=result.payload[:message_id] && Message.find(result.payload[:message_id])
    user.reload
    state=user.attributes.slice('custom_status_emoji','custom_status_text','custom_status_expires_at','dnd_enabled','dnd_until','ooo_until','ooo_note')
    # SQL timestamps pin the storage precision independently of JSON's time renderer.
    state=state.transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? v.utc.strftime('%Y-%m-%d %H:%M:%S.%6N') : v }
    saved=posted && user.saved_items.find_by(message:posted)
    jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.count { |j| j[:job]==Bot::WebhookJob }
    streams=["#{user.to_gid_param}:status","#{user.to_gid_param}:ooo_notice"]
    observations << {message_counts:[count_before,count_after],text:,thread_id:selected_thread&.id,result:result.to_h,state:,message:posted && {id:posted.id,markdown_source:posted.markdown_source,action:posted.action?,streaming:posted.streaming?,thread_id:posted.thread_id,plain:posted.plain_text_body,sound:posted.sound&.name},saved:saved && {status:saved.status,remind_at:saved.remind_at&.utc&.strftime('%Y-%m-%d %H:%M:%S.%6N')},legacy_jobs:jobs,attachment_calls:calls,frames:i==21 ? frames.select { |f| streams.include?(f[:stream]) }.dup : nil}
   end
  end
  cases << {name:,room_id:room.id,rows:,initial:initial.transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? v.utc.strftime('%Y-%m-%d %H:%M:%S.%6N') : v },huddle:config,observations:}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:'2026-09-23T12:00:00Z',cases:)+"\n")
puts "WS8bm2 named slash Rails: #{cases.size}/35 built-in cases; #{cases.sum { |c| c[:observations].size }} observations; actual attachment TracePoint and committed webhook/badge/notice hooks"
