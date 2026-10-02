# Real committed login/status contention and link-CSRF requests against pinned Rails.
require 'json'
require 'openssl'
require 'fileutils'
require 'action_dispatch/testing/integration'
Rails.logger=ActiveSupport::Logger.new($stderr)
ActionController::Base.logger=Rails.logger
ActiveJob::Base.queue_adapter=:test
ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-google-client-secret';ENV['GOOGLE_SIGN_IN_DOMAINS']='smartdata.net'
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
Rails.application.env_config['action_dispatch.show_exceptions']=:all
now=Time.utc(2026,3,2,16);Time.define_singleton_method(:current) { now }
key=OpenSSL::PKey.read(File.binread(File.join(ENV.fetch('PARITY_WORK'),'crates/campfire/src/integrations/google/signing.der')))
jwks=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'crates/campfire/src/integrations/google/test-jwks.json')))
$security_claims=nil
Google::Client.define_singleton_method(:post_token_form) do |**_|
 response=Net::HTTPOK.new('1.1','200','OK');response.instance_variable_set(:@read,true);response.body=JSON.generate({id_token:JWT.encode($security_claims,key,'RS256',{kid:'fixture'})});response
end
Net::HTTP.define_singleton_method(:start) do |host,*_,**_,&block|
 raise "unrecorded host #{host}" unless host=='www.googleapis.com'
 http=Object.new;http.define_singleton_method(:get) do |path|
  raise "unrecorded path #{path}" unless path=='/oauth2/v3/certs'
  response=Net::HTTPOK.new('1.1','200','OK');response.instance_variable_set(:@read,true);response.body=JSON.generate(jwks);response
 end;block.call(http)
end
rows=[]
specs=[:deactivate,:ban].product([false,true]).map { |action,linked|{action:,linked:} }
specs+=%w[anonymous csrf_missing csrf_invalid].map { |kind|{kind:} }
specs.each do |spec|
 database=ActiveRecord::Base.connection_db_config.database
 ActiveRecord::Base.connection.execute('PRAGMA wal_checkpoint(TRUNCATE)');ActiveRecord::Base.connection_pool.disconnect!
 backup="#{database}.auth-security";FileUtils.cp(database,backup)
 original=nil
 begin
  Rails.application.executor.run!(reset:true);Rails.cache.clear;Google::SignIn::KeyStore.clear!
  ActionController::Base.allow_forgery_protection=false
  user=User.find(712064548);user.update_columns(email_address:'race@smartdata.net',google_email_link_allowed:true)
  user.sessions.destroy_all;user.two_factor_credential&.destroy!;GoogleIdentity.delete_all
  GoogleIdentity.create!(user:,subject:'race-member',email:user.email_address,domain:'smartdata.net') if spec[:linked]
  client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
  result=nil
  if spec[:action]
   client.get('/session/new');client.post('/session/google');query=Rack::Utils.parse_query(URI(client.response.location).query)
   $security_claims={'iss'=>'https://accounts.google.com','aud'=>'test-client-id','sub'=>'race-member','email':user.email_address,'email_verified'=>true,'hd'=>'smartdata.net','nonce'=>query['nonce'],'exp'=>now.to_i+3600}
   original=Google::SignIn::AccountLinker.method(:resolve!);intervention=nil
   Google::SignIn::AccountLinker.define_singleton_method(:resolve!) do |claims|
    original.call(claims).tap do |resolved|
     intervention=Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do |connection|
       saved=connection.select_value('PRAGMA busy_timeout');connection.execute('PRAGMA busy_timeout = 0')
       begin;User.find(resolved.id).public_send(spec[:action]);'completed'
       rescue ActiveRecord::StatementInvalid => error;raise unless error.cause.is_a?(SQLite3::BusyException);'retry'
       ensure;connection.execute("PRAGMA busy_timeout = #{saved}")
       end
      end
     end
     raise 'race thread did not finish' unless intervention.join(5)
    end
   end
   client.get('/session/google/callback',params:{state:query['state'],code:'recorded-code'},env:{'REMOTE_ADDR'=>'8.8.8.8'})
   outcome=intervention.value
   user.reload.public_send(spec[:action]) if outcome=='retry'
   client.get('/')
   result={status:user.reload.status,sessions:user.sessions.count,root_status:client.response.status,root_location:client.response.location,contention:outcome}
  else
   if spec[:kind]!='anonymous'
    session=Session.create!(user:,two_factor_verified_at:now)
    request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
    jar=ActionDispatch::Cookies::CookieJar.build(request,{});jar.signed[:session_token]={value:session.token};client.cookies['session_token']=jar[:session_token]
    ActionController::Base.allow_forgery_protection=true
   end
   client.post('/user/profile/google_sign_in_link',headers:spec[:kind]=='csrf_invalid' ? {'X-CSRF-Token'=>'invalid'} : {})
   result={status:client.response.status,location:client.response.location,body:client.response.status==422 ? client.response.body : nil,identities:GoogleIdentity.count}
  end
  rows << {spec:,result:}
 ensure
  Google::SignIn::AccountLinker.define_singleton_method(:resolve!,original) if original
  ActiveRecord::Base.connection_pool.disconnect!;FileUtils.rm_f(["#{database}-wal","#{database}-shm"]);FileUtils.cp(backup,database);FileUtils.rm_f(backup)
 end
end
puts JSON.pretty_generate({reference:'d7c7de92',now:now.to_i,rows:})
