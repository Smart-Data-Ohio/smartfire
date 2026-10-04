# Google portions of the real User#deactivate and connection DELETE; HTTP is recorded.
require 'json'
require 'net/http'
require 'openssl'
require 'action_dispatch/testing/integration'
# The original sampler ran outside the frozen wall clock. Replay its stage-note
# UUID and Message timestamp inputs; Rails still creates and renders the note.
stage_note_input=JSON.parse(File.read(File.join(__dir__,'lifecycle-inputs.json')))
class LifecycleEntropyError < StandardError; end
Message.singleton_class.prepend(Module.new do
  define_method(:current_time_from_proper_timezone) do
    inputs=Thread.current[:google_lifecycle_message_clocks]
    if inputs
      raise LifecycleEntropyError, 'unexpected stage-note clock draw' if inputs.empty?
      Time.at(Rational(inputs.shift,1000)).utc
    else
      super()
    end
  end
end)
Random.singleton_class.prepend(Module.new do
  define_method(:uuid) do
    inputs=Thread.current[:google_lifecycle_message_uuids]
    if inputs
      raise LifecycleEntropyError, 'unexpected stage-note UUID draw' if inputs.empty?
      inputs.shift
    else
      super()
    end
  end
end)
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
ActiveJob::Base.queue_adapter=:test
ENV['GOOGLE_CLIENT_ID']='test-client-id'
ENV['GOOGLE_CLIENT_SECRET']='FAKE-lifecycle-secret'
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
frames=[];calls=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,payload,**options| frames << {stream:,payload:} }
user=User.find(127326141)
http=Object.new
http.define_singleton_method(:post) do |path,body,*args|
  calls << {path:,body:JSON.parse(body),status:user.reload.status,cache:Calendar::MeetingCache.exists?(user_id:user.id)}
  response=Net::HTTPOK.new('1.1','200','recorded')
  response.define_singleton_method(:body) { '{}' }
  response
end
Net::HTTP.define_singleton_method(:start) { |*args,**kwargs,&block| block.call(http) }
protection=ActionController::Base.allow_forgery_protection
ActionController::Base.allow_forgery_protection=false
specs=[
  {name:'deactivate_connected',kind:'deactivate'},
  {name:'deactivate_disconnected',kind:'deactivate',reason:'revoked'},
  {name:'deactivate_unreadable',kind:'deactivate',unreadable:true},
  {name:'deactivate_invalid',kind:'deactivate',invalid:true},
  {name:'deactivate_no_account',kind:'deactivate',account:false},
  {name:'disconnect_meeting',kind:'disconnect'},
  {name:'disconnect_manual_ooo',kind:'disconnect',manual:true},
  {name:'disconnect_no_cache',kind:'disconnect',cache:false}
]
out={reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:[]}
specs.each_with_index do |spec,i|
  Rails.application.executor.run!(reset:true)
  Calendar::PushChannel.delete_all;Calendar::MeetingCache.delete_all;EventCalendarEntry.delete_all;GoogleAccount.delete_all
  user.update_columns(status:0,email_address:'david@example.test',meeting_status_enabled:true,ooo_calendar_enabled:true,ooo_until:spec[:manual] ? now+86400 : nil,ooo_note:spec[:manual] ? 'Own note' : nil,ooo_broadcast:true)
  Current.user=user
  account=GoogleAccount.create!(id:9_800_000_000+i,user:,email:'fixture@example.test',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:now+3600) if spec.fetch(:account,true)
  account.update_columns(disconnected_reason:spec[:reason]) if spec[:reason]
  account.update_columns(email:'') if spec[:invalid]
  GoogleAccount.connection.execute("UPDATE google_accounts SET refresh_token='unreadable' WHERE id=#{account.id}") if spec[:unreadable]
  Calendar::PushChannel.create!(user:,channel_id:'lifecycle-channel',token_digest:'fixture',resource_id:'lifecycle-resource')
  Calendar::MeetingCache.create!(user:,busy_intervals:[[now-60,now+3600]].map { |a,b|[a.iso8601,b.iso8601] },ooo_intervals:[[now-60,now+3600]].map { |a,b|[a.iso8601,b.iso8601] },fetched_at:now,in_meeting_broadcast:true) if spec.fetch(:cache,true)
  entries=EventAttendance.where(user:,response:'going').joins(:event).limit(2).pluck(:event_id)
  entries.each { |id| EventCalendarEntry.create!(event_id:id,user:,google_event_id:"lifecycle-#{id}",synced_at:now) }
  user.reload
  Current.user=user
  calls.clear;frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  error=nil;status=nil
  begin
    if spec[:kind]=='deactivate'
      if i.zero?
        Thread.current[:google_lifecycle_message_clocks]=stage_note_input.fetch('timestamp_milliseconds').dup
        Thread.current[:google_lifecycle_message_uuids]=[stage_note_input.fetch('uuid')]
        begin
          user.deactivate
          raise LifecycleEntropyError, 'unused stage-note entropy inputs' unless Thread.current[:google_lifecycle_message_clocks].empty? && Thread.current[:google_lifecycle_message_uuids].empty?
        ensure
          Thread.current[:google_lifecycle_message_clocks]=nil
          Thread.current[:google_lifecycle_message_uuids]=nil
        end
      else
        user.deactivate
      end
    else
      session=Session.create!(user:,user_agent:'lifecycle-fixture',ip_address:'127.0.0.1',two_factor_verified_at:now)
      req=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
      jar=ActionDispatch::Cookies::CookieJar.build(req,{})
      jar.signed[:session_token]={value:session.token}
      jar.encrypted[:_campfire_session]={value:{session_id:'lifecycle-fixture',sudo_verified_at:now.to_i}}
      cookie="session_token=#{URI.encode_www_form_component(jar[:session_token])}; _campfire_session=#{URI.encode_www_form_component(jar[:_campfire_session])}"
      request=ActionDispatch::Integration::Session.new(Rails.application);request.host! 'campfire.test'
      request.delete('/google/connection',headers:{'HTTP_COOKIE'=>cookie})
      status=request.response.status
    end
  rescue => e
    raise if e.is_a?(LifecycleEntropyError)
    error=e.class.name
  end
  jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j|j[:job].name.start_with?('Calendar::') }.map do |j|
    args=j[:args].dup
    if j[:job]==Calendar::DisconnectCleanupJob
      snapshot=Calendar::DisconnectCleanupJob.decrypt_credentials(args[1])
      args[1]=snapshot&.transform_values { |v|v.is_a?(Time) ? v.iso8601 : v }
    end
    {class:j[:job].name,args:}
  end
  out[:cases] << {spec:,account_id:account&.id,entry_event_ids:entries,error:,status:,calls:calls.dup,frames:frames.dup,jobs:,user_status:user.reload.status,account:GoogleAccount.find_by(user_id:user.id)&.slice('id','disconnected_reason','email'),cache:Calendar::MeetingCache.exists?(user_id:user.id),channel:Calendar::PushChannel.exists?(user_id:user.id),ooo_until:user.ooo_until&.iso8601,ooo_broadcast:user.ooo_broadcast,entry_ids:EventCalendarEntry.where(user:).pluck(:event_id)}
  # Restore the non-direct memberships removed by User#deactivate for the next isolated case.
  user.memberships.without_direct_rooms.delete_all
  [486777696,699448326].each { |id|Room.find(id).memberships.find_or_create_by!(user:) }
end
ActionController::Base.allow_forgery_protection=protection
puts JSON.pretty_generate(out)
