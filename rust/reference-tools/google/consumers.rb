# Production consumers against pinned Rails, recorded Google HTTP only.
require 'json'
require 'net/http'
ActiveJob::Base.queue_adapter = :test
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
ENV['GOOGLE_CLIENT_ID']='test-client-id'
ENV['GOOGLE_CLIENT_SECRET']='FAKE-calendar-consumer-secret'
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
answers=[];calls=[];frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,payload,**options| frames << {stream:,payload:} }
http=Object.new
http.define_singleton_method(:method_missing) do |method,path,*args|
  calls << {method:method.to_s.upcase,path:,body:args.first.is_a?(String) ? (path=='/token' ? URI.decode_www_form(args.first).to_h : JSON.parse(args.first)) : nil}
  status,body=answers.shift || raise("unrecorded Google HTTP forbidden: #{method} #{path}")
  raise Net::ReadTimeout if status=='timeout'
  response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'fixture')
  response.define_singleton_method(:body) { body.to_json }
  response
end
Net::HTTP.define_singleton_method(:start) { |*args,**kwargs,&block| block.call(http) }
user=User.find(127326141);room=Room.find(486777696)
specs = [
  {name:'cancelled_going',kind:'inbound',remote:[200,{status:'cancelled'}]},
  {name:'deleted_maybe',kind:'inbound',local:'maybe',remote:[404,{}]},
  {name:'confirmed_declined',kind:'inbound',local:'declined'},
  {name:'confirmed_going',kind:'inbound'},
  {name:'null_going',kind:'inbound',remote:[200,nil]},
  {name:'no_attendance',kind:'inbound',local:nil,remote:[200,{status:'cancelled'}]},
  {name:'past',kind:'inbound',past:true},
  {name:'cancelled_local',kind:'inbound',cancelled:true},
  {name:'missing_account',kind:'inbound',account:false},
  {name:'disconnected',kind:'inbound',disconnected:true},
  {name:'wrong_scope',kind:'inbound',scopes:'openid'},
  {name:'inactive',kind:'inbound',inactive:true},
  {name:'nonmember',kind:'inbound',nonmember:true},
  {name:'head_declines_followers',kind:'inbound',series:true,remote:[200,{status:'cancelled'}]},
  {name:'follower_declines_locally',kind:'inbound',series:true,index:1,remote:[404,{}]},
  {name:'forbidden_entry_continues',kind:'inbound',extra:true,remote:[403,{error:{message:'denied'}}]},
  {name:'server_error_continues',kind:'inbound',extra:true,remote:[503,{}]},
  {name:'transport_aborts',kind:'inbound',extra:true,remote:['timeout',nil]},
  {name:'quota_aborts',kind:'inbound',extra:true,remote:[429,{}]},
  {name:'revoked_aborts',kind:'inbound',extra:true,remote:[401,{}],refresh:[400,{error:'invalid_grant'}]},
  {name:'provisions',kind:'meet'},
  {name:'synced_copy',kind:'meet',synced:true},
  {name:'not_requested',kind:'meet',requested:false},
  {name:'organizer_unconnected',kind:'meet',account:false},
  {name:'organizer_disconnected',kind:'meet',disconnected:true},
  {name:'existing_link',kind:'meet',link:'https://meet.google.com/existing'},
  {name:'cancelled_meet',kind:'meet',cancelled:true},
  {name:'insert_refused',kind:'meet',insert:[403,{error:{message:'denied'}}]},
  {name:'insert_server_error',kind:'meet',insert:[503,{}]},
  {name:'patch_refused',kind:'meet',remote:[403,{error:{message:'denied'}}]},
  {name:'patch_server_error',kind:'meet',remote:[503,{}]},
  {name:'patch_transport',kind:'meet',remote:['timeout',nil]},
  {name:'patch_quota',kind:'meet',remote:[429,{}]},
  {name:'pending',kind:'meet',remote:[200,{conferenceData:{createRequest:{status:'pending'}}}]},
  {name:'null_conference',kind:'meet',remote:[200,nil]},
  {name:'blank_conference',kind:'meet',remote:[200,{hangoutLink:' '}]},
  {name:'scalar_link',kind:'meet',remote:[200,{hangoutLink:7}]},
  {name:'invalid_event_save',kind:'meet',invalid:true}
]
out={reference:'d7c7de92',now:now.iso8601,cases:[]}
specs.each_with_index do |spec,i|
  Rails.application.executor.run!(reset:true)
  Current.user=user
  EventCalendarEntry.delete_all
  Calendar::PushChannel.delete_all
  GoogleAccount.where(user:).delete_all
  user.update_columns(status:0)
  membership=room.memberships.find_by!(user:)
  event=room.events.create!(id:9_100_000_000+i*10,organizer:user,title:'Calendar consumer',starts_at:now+3600,time_zone:'UTC',meet_link_requested:spec[:kind]=='meet' && spec.fetch(:requested,true),**(spec[:series] ? {recurrence_rule:'weekly',recurrence_until:Date.new(2026,3,16)} : {}))
  rows=event.series_events.order(:starts_at,:id).to_a
  rows.each { |e| spec[:local] == nil && spec.key?(:local) ? e.attendances.delete_all : e.respond!(user,spec.fetch(:local,'going')) }
  target=rows.fetch(spec.fetch(:index,0))
  target.update_column(:meet_link,spec[:link]) if spec.key?(:link)
  target.update_column(:cancelled_at,now) if spec[:cancelled]
  target.update_columns(starts_at:now-3600) if spec[:past]
  extra=room.events.create!(id:event.id+5,organizer:user,title:'Second copy',starts_at:now+7200,time_zone:'UTC') if spec[:extra]
  account=GoogleAccount.create!(user:,email:'fixture@example.test',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:now+3600,scopes:spec.fetch(:scopes,Google::Client::CALENDAR_SCOPE),disconnected_reason:spec[:disconnected] ? 'revoked' : nil) if spec.fetch(:account,true)
  channel=Calendar::PushChannel.create!(user:,channel_id:'fixture-consumer-channel',token_digest:Calendar::PushChannel.digest('fixture-consumer-token'))
  entry=EventCalendarEntry.create!(id:9_200_000_000+i*10,event:target,user:,google_event_id:Calendar::EntrySync.google_event_id_for(target.id,user.id),synced_at:spec[:kind]=='inbound'||spec[:synced] ? now : nil) if spec[:kind]=='inbound'||spec[:synced]
  EventCalendarEntry.create!(id:9_200_000_000+i*10+1,event:extra,user:,google_event_id:Calendar::EntrySync.google_event_id_for(extra.id,user.id),synced_at:now) if extra
  target.update_column(:title,'') if spec[:invalid]
  user.update_columns(status:1) if spec[:inactive]
  membership.delete if spec[:nonmember]
  before=rows.map { |e| {id:e.id,starts_at:e.reload.starts_at.iso8601,ends_at:e.ends_at&.iso8601,cancelled:e.cancelled?,requested:e.meet_link_requested?,link:e.meet_link,response:e.response_for(user)} }
  before << {id:extra.id,starts_at:extra.starts_at.iso8601,ends_at:nil,cancelled:false,requested:false,link:nil,response:extra.response_for(user)} if extra
  answers.replace([spec.fetch(:remote,[200,spec[:kind]=='meet' ? {hangoutLink:'https://meet.google.com/abc-defg-hij'} : {status:'confirmed'}])])
  answers.unshift(spec.fetch(:insert,[200,{}])) if spec[:kind]=='meet'
  answers << spec[:refresh] if spec[:refresh]
  answers << [200,{status:'confirmed'}] if spec[:extra]
  target.referencing_messages.each { |message|message.update_column(:client_message_id,"consumer-#{spec[:name]}") }
  announcement_id=target.referencing_messages.first&.id
  calls.clear
  frames.clear
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  error=nil
  begin
    spec[:kind]=='inbound' ? Calendar::InboundSync.sync(user.id) : Calendar::MeetLink.provision!(target)
  rescue => e
    error=e.class.name
  end
  jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j|j[:job].name.start_with?('Calendar::') }.map { |j| {class:j[:job].name,args:j[:args]} }
  out[:cases] << {spec:,events:before,entry_id:entry&.id,error:,calls:calls.dup,link:target.reload.meet_link,responses:before.map { |e|Event.find(e[:id]).response_for(user) },jobs:,channel_error:channel.reload.last_error,connected:account&.reload&.connected?,announcement_id:,frames:frames.dup}
  room.memberships.create!(user:,involvement:membership.involvement) if spec[:nonmember]
  user.update_columns(status:0)
end
puts JSON.pretty_generate(out)
