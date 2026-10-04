# Real connection producers and organizer/user isolation. Jobs remain in the test queue.
require 'json'
require 'fileutils'
require 'action_dispatch/testing/integration'
load File.join(ENV.fetch("PARITY_WORK"), "reference-tools/google/google_calendar_test_helper.rb")
helper=Object.new.extend(GoogleCalendarTestHelper)
Rails.logger=ActiveSupport::Logger.new($stderr);ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-google-client-secret'
now=Time.utc(2026,3,2,16);Time.define_singleton_method(:current) {now}
Google::Client.define_singleton_method(:post_token_form) do |**params|
  response=Net::HTTPOK.new('1.1','200','recorded');response.define_singleton_method(:body) { JSON.generate({access_token:'new-access-token',refresh_token:'new-refresh-token',expires_in:3600,id_token:helper.send(:google_id_token),scope:"#{Google::Client::CALENDAR_SCOPE} #{Google::Client::DRIVE_SCOPE}"}) };response
end
Net::HTTP.define_singleton_method(:start) { |*|raise 'unrecorded Google HTTP' }
rows=[]
%w[callback callback_meeting callback_ooo disconnect_links disconnect_other disconnect_entries disconnect_identity].each do |name|
  database=ActiveRecord::Base.connection_db_config.database;ActiveRecord::Base.connection.execute('PRAGMA wal_checkpoint(TRUNCATE)');ActiveRecord::Base.connection_pool.disconnect!
  snapshot="#{database}.connection-cases";FileUtils.cp(database,snapshot)
  begin
    Rails.application.executor.run!(reset:true)
    david=User.find(127326141);jason=User.find(149087659)
    GoogleAccount.delete_all;GoogleIdentity.delete_all;Calendar::PushChannel.delete_all;EventCalendarEntry.delete_all
    david.update_columns(meeting_status_enabled:name=='callback_meeting',ooo_calendar_enabled:name=='callback_ooo')
    events=Event.where(organizer:david).order(:id).limit(2).to_a;raise 'missing organizer events' if events.length!=2
    unless name.start_with?('callback')
      helper.connect_google!(david);helper.connect_google!(jason) if name=='disconnect_entries'
      events.each { |e|e.update_columns(meet_link_requested:true,meet_link:'https://meet.google.com/abc-defg-hij') } if %w[disconnect_links disconnect_other].include?(name)
      events.last.update_columns(organizer_id:jason.id) if name=='disconnect_other'
      if name=='disconnect_entries'
        EventCalendarEntry.create!(event:events.first,user:david,google_event_id:'mine-fixture')
        EventCalendarEntry.create!(event:events.first,user:jason,google_event_id:'theirs-fixture')
      end
      GoogleIdentity.create!(user:david,subject:'connection-member',email:'login@smartdata.net',domain:'smartdata.net') if name=='disconnect_identity'
    end
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear;AuditLog.delete_all
    session=Session.create!(user:david,user_agent:'connection-fixture',ip_address:'127.0.0.1',two_factor_verified_at:now)
    request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
    jar=ActionDispatch::Cookies::CookieJar.build(request,{});jar.signed[:session_token]={value:session.token};jar.encrypted[:_campfire_session]={value:{session_id:'connection-fixture',sudo_verified_at:now.to_i}}
    client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test';client.cookies['session_token']=jar[:session_token];client.cookies['_campfire_session']=jar[:_campfire_session]
    if name.start_with?('callback')
      client.post('/google/connect');state=Rack::Utils.parse_query(URI(client.response.location).query)['state'];client.get('/google/callback',params:{state:,code:'fixture'})
    else
      client.delete('/google/connection')
    end
    jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.select { |job|job[:job].name.start_with?('Calendar::') }.map { |job|{class:job[:job].name,args:job[:job]==Calendar::DisconnectCleanupJob ? [job[:args][0]] : job[:args]} }.sort_by(&:to_json)
    account=GoogleAccount.find_by(user_id:david.id)
    rows<<{name:,event_ids:events.map(&:id),result:{status:client.response.status,location:client.response.location,flash:client.request.flash.to_hash,jobs:,accounts:GoogleAccount.order(:user_id).pluck(:user_id),identity:GoogleIdentity.order(:user_id).pluck(:user_id,:subject),entries:EventCalendarEntry.order(:user_id).pluck(:user_id,:google_event_id),links:events.map { |e|e.reload.attributes.slice('id','organizer_id','meet_link','meet_link_requested') },account:account && {email:account.email,scopes:account.scopes,access_token:account.access_token,refresh_token:account.refresh_token,calendar:account.calendar?,drive:account.drive?},audits:AuditLog.order(:id).pluck(:action)}}
  ensure
    ActiveRecord::Base.connection_pool.disconnect!;FileUtils.rm_f(["#{database}-wal","#{database}-shm"]);FileUtils.cp(snapshot,database);FileUtils.rm_f(snapshot)
  end
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:now.to_i,rows:})
