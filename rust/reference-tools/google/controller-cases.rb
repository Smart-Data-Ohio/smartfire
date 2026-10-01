# Complete request/flash/audit/state observations; every outbound exchange is recorded.
require 'json'
require 'openssl'
require 'action_dispatch/testing/integration'
require 'fileutils'
Rails.logger=ActiveSupport::Logger.new($stderr)
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = false
Rails.application.routes.default_url_options.merge!(host: 'campfire.test', protocol: 'http')
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_) { 'NONCE' }
ApplicationController.prepend(Module.new do
  def form_authenticity_token(form_options:{})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end)
ENV['VAPID_PUBLIC_KEY']='';Rails.configuration.x.vapid.public_key=nil
BASE = Time.utc(2026,3,2,16)
$controller_now = BASE
Time.define_singleton_method(:current) { $controller_now }
KEY = OpenSSL::PKey.read(File.binread(File.join(ENV.fetch('PARITY_WORK'),'crates/campfire/src/integrations/google/signing.der')))
ROTATED_KEY = OpenSSL::PKey.read(File.binread(File.join(ENV.fetch('PARITY_WORK'),'crates/campfire/src/integrations/google/rotated-signing.der')))
ROTATED_JWKS = { 'keys' => [{ 'kty'=>'RSA','kid'=>'rotated','n'=>Base64.urlsafe_encode64(ROTATED_KEY.n.to_s(2),padding:false),'e'=>Base64.urlsafe_encode64(ROTATED_KEY.e.to_s(2),padding:false) }] }
JWKS = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'crates/campfire/src/integrations/google/test-jwks.json')))
$controller_calls = []
$controller_token_payload = nil
$controller_key_payload = JWKS
Google::Client.define_singleton_method(:post_token_form) do |**params|
  raise 'bad recorded token request' unless params[:grant_type] == 'authorization_code' && params[:code_verifier].present?
  $controller_calls << 'token'
  raise Net::ReadTimeout if $controller_transport
  response = Net::HTTPOK.new('1.1','200','OK')
  response.instance_variable_set(:@read,true)
  response.body = JSON.generate($controller_token_payload)
  response
end
Net::HTTP.define_singleton_method(:start) do |host, *args, **options, &block|
  raise "unrecorded Google host: #{host}" unless host == 'www.googleapis.com'
  http = Object.new
  http.define_singleton_method(:get) do |path|
    raise "unrecorded Google path: #{path}" unless path == '/oauth2/v3/certs'
    $controller_calls << 'keys'
    response = Net::HTTPOK.new('1.1','200','OK'); response.instance_variable_set(:@read,true)
    response.body = JSON.generate($controller_key_payload); response
  end
  block.call(http)
end

def sign_in(client,user)
  session=Session.create!(user:,user_agent:'controller-cases-fixture',ip_address:'127.0.0.1',two_factor_verified_at:Time.current)
  req=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'))
  jar=ActionDispatch::Cookies::CookieJar.build(req,{})
  jar.signed[:session_token]={value:session.token}
  client.cookies['session_token']=jar[:session_token]
end

def counts
  [User.count,Session.count,GoogleIdentity.count,GoogleAccount.count]
end

def observation(client,before,call_before)
  user=User.find_by!(name:'Kevin')
  {
    status:client.response.status,location:client.response.location,flash:client.response.redirect? ? client.request.flash.to_hash : {},
    device_count:TwoFactorRememberedDevice.where(user_id:127326141).count,
    backup_count:TwoFactorBackupCode.joins(:two_factor_credential).where(two_factor_credentials:{user_id:127326141},used_at:nil).count,
    credential_enabled:User.find(127326141).two_factor_enabled?,
    calls:$controller_calls[call_before..],delta:counts.zip(before).map { |a,b| a-b },
    markers:client.request.session.to_h.slice('two_factor_reauthenticated_at','sudo_verified_at'),
    flow_present:client.request.session[:google_sign_in_request].present?,
    calendar_users:GoogleAccount.order(:user_id).pluck(:user_id),
    identities:GoogleIdentity.order(:user_id).map { |i|i.attributes.slice('user_id','subject','email','domain') },
    email:user.email_address,password_preserved:user.password_digest==$controller_password,
    audits:AuditLog.order(:id).map { |a|a.attributes.slice('action','actor_id','target_id','target_type','target_label','details') }
  }
end

def lifecycle_observation
  target=GoogleIdentity.find_by(subject:'controller-member')&.user || User.find_by(email_address:'new-member@smartdata.net') || User.find_by(email_address:'new-member@cnbssoftware.com') || User.find(712064548)
  {
    user:target.attributes.slice('id','name','email_address','role','status'),
    password_present:target.password_digest.present?,
    email_self_changed:target.email_self_changed_at.present?,google_email_link_allowed:target.google_email_link_allowed?,
    memberships:target.memberships.order(:room_id).pluck(:room_id),
    history_preserved:Message.where(creator_id:712064548).order(:id).pluck(:id,:room_id,:creator_id)==$controller_history
  }
end

specs=[]
%w[reauth sudo].each do |purpose|
  %w[success wrong_subject stale missing_auth before_auth_boundary at_auth_boundary after_auth_boundary wrong_nonce wrong_domain unavailable cancelled expired_flow replay unlinked unconfigured signed_out other_member].each { |scenario|specs << {purpose:,scenario:} }
end
%w[success wrong_nonce wrong_domain subject_taken already_linked signed_out other_member unavailable cancelled expired_flow anonymous unconfigured].each { |scenario|specs << {purpose:'link',scenario:} }
[nil,[],42,'unexpected',{'id_token'=>[]},{'id_token'=>{}}].each { |payload| specs << {purpose:'sign_in',scenario:'token_shape',payload:} }
[nil,[],42,{'keys'=>nil},{'keys'=>'unexpected'},{'keys'=>[nil,42,'invalid',{'kty'=>'RSA','kid'=>[],'n'=>{},'e'=>42}]}].each { |payload| specs << {purpose:'sign_in',scenario:'key_shape',payload:} }
%w[consume_backup consume_device consume_all_devices consume_disable before_reauth_expiry at_reauth_expiry after_reauth_expiry wrong_credential_preserves].each { |scenario|specs << {purpose:'reauth',scenario:} }
%w[legacy_password provision provision_secondary provision_secondary_hosted provision_org external_password immutable_email deactivated banned bot retained_deactivated retained_banned different_subject self_changed admin_allowed linking_disabled policy_changed].each { |scenario|specs << {purpose:'sign_in',scenario:,lifecycle:true} }
specs += %w[success wrong_subject stale missing_auth].map { |scenario|{purpose:'sudo',scenario:,continuation:true} }
specs += %w[join_signup provision_self_change calendar_only].map { |scenario| {purpose:'sign_in',scenario:,lifecycle:true} }
specs << {purpose:'sign_in',scenario:'predecessor',lifecycle:true}
specs << {purpose:'sign_in',scenario:'rotation',lifecycle:true}
specs += %w[forged_state missing_state replay unconfigured_password].map { |scenario| {purpose:'sign_in',scenario:} }
rows=[]
specs.each_with_index do |spec,index|
  # Restore the isolated reference database between cases. An enclosing rollback
  # transaction would suppress the request's real after_commit membership effects.
  database=ActiveRecord::Base.connection_db_config.database
  ActiveRecord::Base.connection.execute('PRAGMA wal_checkpoint(TRUNCATE)')
  ActiveRecord::Base.connection_pool.disconnect!
  snapshot="#{database}.controller-cases-backup"
  FileUtils.cp(database,snapshot)
  begin
    Rails.application.executor.run!(reset:true)
    $controller_now=BASE
    ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-google-client-secret'
    ENV['GOOGLE_SIGN_IN_DOMAINS']=%w[unconfigured unconfigured_password].include?(spec[:scenario]) ? '' : 'smartdata.net,cnbssoftware.com'
    ENV['GOOGLE_SIGN_IN_DOMAINS']='EXAMPLE.ORG' if spec[:scenario]=='provision_org'
    Rails.cache.clear
    Google::SignIn::KeyStore.clear!
    GoogleIdentity.delete_all;AuditLog.delete_all
    kevin=User.find_by!(name:'Kevin');david=User.find_by!(name:'David');jz=User.find_by!(name:'JZ')
    $controller_password=kevin.password_digest
    purpose=spec[:purpose];scenario=spec[:scenario]
    if spec[:lifecycle]
      kevin.update_columns(email_address:scenario=='external_password' ? 'legacy@external.test' : 'legacy@smartdata.net',role:1,google_email_link_allowed:true,email_self_changed_at:nil)
      kevin.update_columns(status:1) if %w[deactivated retained_deactivated].include?(scenario)
      kevin.update_columns(status:2) if %w[banned retained_banned].include?(scenario)
      kevin.update_columns(role:2) if scenario=='bot'
      kevin.update_columns(status:1,email_address:'legacy-deactivated-fixture@smartdata.net') if scenario=='predecessor'
      kevin.update_columns(email_self_changed_at:BASE,google_email_link_allowed:false) if %w[self_changed admin_allowed].include?(scenario)
      kevin.update_columns(google_email_link_allowed:false) if scenario=='linking_disabled'
      kevin.update!(email_self_changed_at:nil,google_email_link_allowed:true) if scenario=='admin_allowed'
      if %w[immutable_email retained_deactivated retained_banned different_subject].include?(scenario)
        GoogleIdentity.create!(user:kevin,subject:scenario=='different_subject' ? 'controller-old' : 'controller-member',email:'legacy@smartdata.net',domain:'smartdata.net')
      end
      ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=9000000000 WHERE name='users'")
      $controller_history=Message.where(creator_id:kevin.id).order(:id).pluck(:id,:room_id,:creator_id)
      raise 'missing authored history fixture' if $controller_history.empty?
    end
    actor=purpose=='link' || purpose=='sign_in' ? kevin : david
    actor.update_columns(password_digest:nil) if spec[:continuation]
    subject='controller-member';email=purpose=='link' ? 'kevin.w@smartdata.net' : 'david@smartdata.net'
    email=case scenario
      when 'provision','provision_self_change' then 'new-member@smartdata.net'
      when 'join_signup' then 'newhire@smartdata.net'
      when 'calendar_only' then 'david@smartdata.net'
      when 'provision_secondary','provision_secondary_hosted' then 'new-member@cnbssoftware.com'
      when 'provision_org' then 'new-member@example.org'
      when 'external_password' then 'legacy@external.test'
      when 'immutable_email' then 'changed@smartdata.net'
      else 'LEGACY@smartdata.net'
    end if spec[:lifecycle]
    if %w[reauth sudo].include?(purpose) && scenario!='unlinked'
      GoogleIdentity.create!(user:actor,subject:,email:,domain:'smartdata.net')
    elsif scenario=='already_linked'
      GoogleIdentity.create!(user:actor,subject:'controller-old',email:'kevin@smartdata.net',domain:'smartdata.net')
    elsif scenario=='subject_taken'
      GoogleIdentity.create!(user:jz,subject:,email:'jz@smartdata.net',domain:'smartdata.net')
    end
    client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
    sign_in(client,actor) unless purpose=='sign_in' || scenario=='anonymous'
    device = nil
    if %w[consume_device consume_all_devices].include?(scenario)
      device, = TwoFactorRememberedDevice.create_for!(actor,user_agent:"Controller fixture",ip_address:"127.0.0.1")
    end
    if scenario=='join_signup'
      code=Account.first.join_code
      client.get("/join/#{code}");client.post("/join/#{code}",params:{user:{name:'Pre-claimer',email_address:'newhire@smartdata.net',password:'secret123456'}});client.delete('/session')
    end
    if scenario=='calendar_only'
      GoogleAccount.where(user:david).delete_all
      Rails.application.executor.run!(reset:true)
      GoogleAccount.create!(user:david,email:'david@smartdata.net',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:BASE+3600)
    end
    initial=nil
    if purpose=='sign_in'
      client.get('/session/new')
      initial={status:client.response.status,google_mark:client.response.body.include?('Sign in with Google'),domain_sentence:client.response.body[/Google sign-in for (.*?) accounts/,1]}
    end
    prompt=nil;gate=nil
    if spec[:continuation]
      client.delete('/fizzy/connection')
      gate={status:client.response.status,location:client.response.location}
      ActionController::Base.allow_forgery_protection=true
      client.get('/sudo/new');prompt={status:client.response.status,body:client.response.body}
      ActionController::Base.allow_forgery_protection=false
    end
    before=counts
    path={'reauth'=>'/two_factor_reauthentication','sudo'=>'/sudo/google','link'=>'/user/profile/google_sign_in_link','sign_in'=>'/session/google'}.fetch(purpose)
    $controller_calls=[];$controller_transport=false;$controller_key_payload=JWKS
    client.post(path)
    start={status:client.response.status}
    start[:page]=initial if initial
    start[:gate]=gate if gate
    start[:prompt]=prompt if prompt
    query=client.response.location && Rack::Utils.parse_query(URI(client.response.location).query)
    if query && query['state']
      flow=client.request.session[:google_sign_in_request]
      start[:bound_entropy]={nonce:query['nonce']==flow['nonce'] && flow['nonce'].length==32,state:Rails.application.message_verifier('google_sign_in_state').verify(query['state'])==flow['state'],pkce:query['code_challenge']==Base64.urlsafe_encode64(Digest::SHA256.digest(flow['verifier']),padding:false)}
      start[:authorize]=query.slice('scope','code_challenge_method','prompt','max_age','redirect_uri')
      if scenario=='signed_out' || scenario=='other_member'
        client.delete('/session')
        sign_in(client,jz) if scenario=='other_member'
      client.get(scenario=='other_member' ? '/users/me/profile' : '/session/new') if %w[signed_out other_member].include?(scenario)
      end
      $controller_now=BASE+601 if scenario=='expired_flow'
      ENV['GOOGLE_SIGN_IN_DOMAINS']='cnbssoftware.com' if scenario=='policy_changed'
      auth_offset={'stale'=>-360,'before_auth_boundary'=>-331,'at_auth_boundary'=>-330,'after_auth_boundary'=>-329}.fetch(scenario,0)
      claims={'iss'=>'https://accounts.google.com','aud'=>'test-client-id','sub'=>scenario=='wrong_subject' ? 'controller-attacker' : subject,'email'=>email,'email_verified'=>true,'hd'=>scenario=='wrong_domain' ? 'wrong.test' : 'smartdata.net','name'=>'Fixture member','nonce'=>scenario=='wrong_nonce' ? 'forged' : query['nonce'],'exp'=>BASE.to_i+3600,'auth_time'=>BASE.to_i+auth_offset}
      claims['hd']='cnbssoftware.com' if scenario=='provision_secondary'
      claims['hd']='example.org' if scenario=='provision_org'
      claims['hd']='external.test' if scenario=='external_password'
      claims.delete('auth_time') if scenario=='missing_auth'
      $controller_token_payload={'access_token'=>'signin-access-token','refresh_token'=>'signin-refresh-token','id_token'=>JWT.encode(claims,KEY,'RS256',{kid:'fixture'})}
      $controller_token_payload=spec[:payload] if scenario=='token_shape'
      $controller_key_payload=spec[:payload] if scenario=='key_shape'
      $controller_transport=scenario=='unavailable'
      if scenario=='rotation'
        Google::SignIn::KeyStore.public_key_for('fixture')
        $controller_key_payload=ROTATED_JWKS
        $controller_token_payload={'access_token'=>'signin-access-token','refresh_token'=>'signin-refresh-token','id_token'=>JWT.encode(claims,ROTATED_KEY,'RS256',{kid:'rotated'})}
      end
      call_before=0
      ActionController::Base.allow_forgery_protection=true
      client.get('/session/google/callback',params:{state:scenario=='forged_state' ? 'forged' : scenario=='missing_state' ? nil : query['state'],code:'fixture-code',**(scenario=='cancelled' ? {error:'access_denied'} : {})})
      ActionController::Base.allow_forgery_protection=false
      if scenario=='provision_self_change'
        Rails.application.executor.run!(reset:true)
        user=GoogleIdentity.find_by!(subject:'controller-member').user
        user.sessions.order(:id).last.mark_two_factor_verified!
        client.put('/users/me/profile',params:{user:{email_address:'changed@smartdata.net'}})
        raise 'profile change failed' unless client.response.status==302 && user.reload.email_self_changed_at.present?
        client.delete('/session');client.get('/session/new');client.post('/session/google')
        query=Rack::Utils.parse_query(URI(client.response.location).query)
        claims['nonce']=query['nonce'];$controller_token_payload={'access_token'=>'signin-access-token','id_token'=>JWT.encode(claims,KEY,'RS256',{kid:'fixture'})}
        client.get('/session/google/callback',params:{state:query['state'],code:'fixture-code'})
      end
      result=observation(client,before,call_before)
      if %w[token_shape key_shape].include?(scenario)
        ActionController::Base.allow_forgery_protection=true
        client.follow_redirect!
        ActionController::Base.allow_forgery_protection=false
        result[:follow]={status:client.response.status,body:client.response.body}
      end
      if spec[:continuation]
        result[:body]=client.response.body if client.response.status==200
        client.delete('/fizzy/connection');result[:protected]=observation(client,before,$controller_calls.length)
        client.get('/users/me/profile');result[:same_member]={status:client.response.status,email_visible:client.response.body.include?('david@37signals.com'),identity_owner:GoogleIdentity.find_by!(subject:'controller-member').user_id}
      end
      if spec[:lifecycle]
        result[:lifecycle]=lifecycle_observation
        if %w[legacy_password provision provision_secondary external_password].include?(scenario)
          client.get(client.response.location)
          client.delete('/session')
          client.get('/session/new')
          call_before=$controller_calls.length
          client.post('/session',params:{email_address:scenario=='legacy_password' ? 'legacy@smartdata.net' : email,password:'secret123456'})
          result[:password_login]=observation(client,before,call_before)
          result[:password_login][:lifecycle]=lifecycle_observation
        end
      end
      if purpose=='reauth'
        # The UI follows the callback redirect before submitting its protected form.
        client.get(client.response.location)
        $controller_now=BASE+{'before_reauth_expiry'=>599,'at_reauth_expiry'=>600,'after_reauth_expiry'=>601}.fetch(scenario,0)
        actions=case scenario
        when 'consume_device' then [['delete',"/two_factor_remembered_devices/#{device.id}",{}]]
        when 'consume_all_devices' then [['delete','/two_factor_remembered_devices',{}]]
        when 'consume_disable' then [['delete','/two_factor_setup',{}]]
        when 'consume_backup' then [['post','/two_factor_backup_codes',{}],['post','/two_factor_backup_codes',{}]]
        when 'wrong_credential_preserves' then [['post','/two_factor_backup_codes',{reauth:'wrong-credential'}],['post','/two_factor_backup_codes',{}]]
        else [['post','/two_factor_backup_codes',{}]]
        end
        result[:protected]=actions.map do |method,path,params|
          call_before=$controller_calls.length
          client.public_send(method,path,params:)
          observation(client,before,call_before)
        end
      end
      if scenario=='replay'
        call_before=$controller_calls.length
        client.get('/session/google/callback',params:{state:query['state'],code:'fixture-code'})
        result[:replay]=observation(client,before,call_before)
      end
    else
      result=observation(client,before,0)
      if scenario=='unconfigured_password'
        client.get('/session/new')
        client.post('/session',params:{email_address:kevin.email_address,password:'secret123456'})
        result[:password_login]=observation(client,before,0)
      end
    end
    rows << {index:,spec:,actor_id:actor.id,jz_id:jz.id,start:,result:}
  rescue => error
    warn error.full_message
    raise
  ensure
    ActiveRecord::Base.connection_pool.disconnect!
    FileUtils.rm_f(["#{database}-wal","#{database}-shm"])
    FileUtils.cp(snapshot,database)
    FileUtils.rm_f(snapshot)
  end
end
puts JSON.pretty_generate({reference:'d7c7de92',now:BASE.to_i,rows:,rotated_jwks:ROTATED_JWKS,identity_token_columns:GoogleIdentity.column_names & %w[access_token refresh_token],filtered_code:ActiveSupport::ParameterFilter.new(Rails.application.config.filter_parameters).filter_param('code','private-code-fixture')})
