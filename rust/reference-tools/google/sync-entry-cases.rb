# EntrySync consumer observations: full recorded requests, durable retry contract and stored rows.
require 'json'
require 'net/http'
require 'fileutils'
ActiveJob::Base.queue_adapter=:test
Rails.logger=ActiveSupport::Logger.new($stderr);ActiveJob::Base.logger=Rails.logger
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-calendar-client-secret'
now=Time.utc(2026,3,2,16);Time.define_singleton_method(:current) { now }
answers=[];calls=[]
http=Object.new
http.define_singleton_method(:method_missing) do |method,path,*args|
 calls << {method:method.to_s.upcase,path:,body:args.first.is_a?(String) ? (path=='/token' ? URI.decode_www_form(args.first).to_h : JSON.parse(args.first)) : nil,access_token:args.last.is_a?(Hash) ? args.last['Authorization']&.delete_prefix('Bearer ') : nil}
 status,body=answers.shift || raise("unrecorded Google call #{method} #{path}")
 raise Net::OpenTimeout if status=='timeout'
 raise Object.const_get(body), 'recorded private transport detail' if status=='transport'
 response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'fixture');response.define_singleton_method(:body) {body.to_json};response
end
Net::HTTP.define_singleton_method(:start) { |*args,**kwargs,&block| raise 'unexpected host' unless %w[www.googleapis.com oauth2.googleapis.com].include?(args[0]);block.call(http) }
quota={error:{errors:[{reason:'rateLimitExceeded'}]}}
specs=[
{name:'going'}, {name:'venue'}, {name:'no_end'},
{name:'maybe',steps:[{}, {response:'maybe'}]},
{name:'decline',steps:[{}, {response:'declined',answers:[[200,{}]]}]},
{name:'time_change',steps:[{}, {time_change:true}]},
{name:'cancel',steps:[{}, {cancel:true,answers:[[200,{}]]}]},
{name:'leave',steps:[{}, {leave:true,answers:[[200,{}]]}]},
{name:'missing_account',account:false},{name:'disconnected',disconnected:true,existing:true},
{name:'invalid_grant',expired:true,answers:[[400,{error:'invalid_grant'}]]},
{name:'two_runs',steps:[{},{}]},
{name:'conflict_twice',steps:[{answers:[[409,{}],[200,{}]]},{}]},
{name:'insert_conflict',answers:[[409,{}],[200,{}]]},
{name:'update_missing',existing:true,synced:true,answers:[[404,{}],[200,{}]]},
{name:'delete_404',existing:true,response:'declined',answers:[[404,{}]]},
{name:'insert_500',answers:[[500,{}]]},
{name:'transport_job',job:true,answers:[['timeout',nil]]},
{name:'transport_reconciler',answers:[['timeout',nil]]},
{name:'transport_read',job:true,answers:[['transport','Net::ReadTimeout']]},
{name:'transport_write',job:true,answers:[['transport','Net::WriteTimeout']]},
{name:'transport_socket',job:true,answers:[['transport','SocketError']]},
{name:'transport_tls',job:true,answers:[['transport','OpenSSL::SSL::SSLError']]},
{name:'transport_refused',job:true,answers:[['transport','Errno::ECONNREFUSED']]},
{name:'transport_reset',job:true,answers:[['transport','Errno::ECONNRESET']]},
{name:'transport_closed',job:true,answers:[['transport','EOFError']]},
{name:'transport_malformed',job:true,answers:[['transport','Net::HTTPBadResponse']]},

{name:'quota_429',job:true,answers:[[429,{}]]},
{name:'quota_403',job:true,answers:[[403,quota]]},
{name:'permission_403',job:true,answers:[[403,{error:{errors:[{reason:'forbidden'}]}}]]},
{name:'delete_transport',job:true,existing:true,response:'declined',answers:[['timeout',nil]]},
{name:'resurrect',steps:[{}, {response:'declined',answers:[[200,{}]]},{response:'going',answers:[[409,{}],[200,{}]]}]},
{name:'delete_410',existing:true,response:'declined',answers:[[410,{}]]},
{name:'missing_scope',scope:'openid email'}, {name:'lost_scope',existing:true,scope:'openid email'},
{name:'unreadable',existing:true,unreadable:true},
{name:'delete_500',existing:true,response:'declined',answers:[[500,{}]]},
{name:'missing_event',event_id:0},{name:'missing_user',user_id:0}
]
rows=[]
specs.each do |spec|
 database=ActiveRecord::Base.connection_db_config.database
 ActiveRecord::Base.connection.execute('PRAGMA wal_checkpoint(TRUNCATE)');ActiveRecord::Base.connection_pool.disconnect!
 backup="#{database}.sync-entry";FileUtils.cp(database,backup)
 begin
  Rails.application.executor.run!(reset:true)
  user=User.find(127326141);event=Event.find(ActiveRecord::FixtureSet.identify('launch_party'));GoogleAccount.delete_all;EventCalendarEntry.delete_all
  event.update_column(:ends_at,nil) if spec[:name]=='no_end'
  if spec[:name]=='venue'
   venue=Rooms::Voice.create_for({name:'Lounge',creator:user},users:[user]);event.update_column(:venue_room_id,venue.id)
  end
  account=GoogleAccount.create!(user:,email:'david@smartdata.net',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:spec[:expired] ? now-1 : now+3600,scopes:spec.fetch(:scope,Google::Client::CALENDAR_SCOPE),disconnected_reason:spec[:disconnected] ? 'revoked' : nil) if spec.fetch(:account,true)
  if spec[:unreadable]
   ActiveRecord::Base.connection.execute("UPDATE google_accounts SET access_token='broken-AR-ciphertext' WHERE id=#{account.id}");user.reload
  end
  EventCalendarEntry.create!(event:,user:,google_event_id:Calendar::EntrySync.google_event_id_for(event.id,user.id),synced_at:spec[:synced] ? now-86400 : nil,last_error:'old error') if spec[:existing]
  initial=event.attributes.slice('id','room_id','title','description','starts_at','ends_at','time_zone','venue_room_id')
  observations=[]
  spec.fetch(:steps,[spec]).each do |step|
   event.attendances.find_by!(user:).update_column(:response,step[:response] || spec[:response]) if step[:response] || spec[:response]
   event.update_column(:starts_at,event.starts_at+1800) if step[:time_change]
   event.update_column(:cancelled_at,now) if step[:cancel]
   event.room.memberships.find_by!(user:).delete if step[:leave]
   answers.replace(step.fetch(:answers,spec.fetch(:answers,[[200,{}]])));calls.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
   error=nil
   begin
    args=[spec.fetch(:event_id,event.id),spec.fetch(:user_id,user.id)]
    spec[:job] ? Calendar::SyncEntryJob.perform_now(*args) : Calendar::EntrySync.sync(*args)
   rescue => e;error=e.class.name
   end
   entry=EventCalendarEntry.find_by(event:,user:)
   jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j|j[:job].name.start_with?('Calendar::') }.map { |j|{class:j[:job].name,args:j[:args]} }
   observations << {calls:calls.dup,error:,entry:entry&.attributes&.slice('event_id','user_id','google_event_id','synced_at','last_error'),disconnected:account&.reload&.disconnected_reason,jobs:}
  end
  rows << {spec:,event:initial,venue:event.venue&.attributes&.slice('id','name'),observations:}
 ensure
  ActiveRecord::Base.connection_pool.disconnect!;FileUtils.rm_f(["#{database}-wal","#{database}-shm"]);FileUtils.cp(backup,database);FileUtils.rm_f(backup)
 end
end
puts JSON.pretty_generate({reference:'d7c7de92',now:now.iso8601,rows:})
warn "Pinned Rails EntrySync: #{rows.size} consumer scenarios; recorded HTTP only"
