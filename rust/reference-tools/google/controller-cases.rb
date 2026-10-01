# Complete request/flash/audit/state observations; every outbound exchange is recorded.
require 'json'
require 'openssl'
require 'action_dispatch/testing/integration'
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = false
Rails.application.routes.default_url_options.merge!(host: 'campfire.test', protocol: 'http')
BASE = Time.utc(2026,3,2,16)
$controller_now = BASE
Time.define_singleton_method(:current) { $controller_now }
KEY = OpenSSL::PKey.read(File.binread(File.join(ENV.fetch('PARITY_WORK'),'crates/campfire/src/integrations/google/signing.der')))
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
    identities:GoogleIdentity.order(:user_id).map { |i|i.attributes.slice('user_id','subject','email','domain') },
    email:user.email_address,password_preserved:user.password_digest==$controller_password,
    audits:AuditLog.order(:id).map { |a|a.attributes.slice('action','actor_id','target_id','target_type','target_label','details') }
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
rows=[]
specs.each_with_index do |spec,index|
  ActiveRecord::Base.transaction(requires_new:true) do
    $controller_now=BASE
    ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-google-client-secret'
    ENV['GOOGLE_SIGN_IN_DOMAINS']=spec[:scenario]=='unconfigured' ? '' : 'smartdata.net,cnbssoftware.com'
    Rails.cache.clear
    Google::SignIn::KeyStore.clear!
    GoogleIdentity.delete_all;AuditLog.delete_all
    kevin=User.find_by!(name:'Kevin');david=User.find_by!(name:'David');jz=User.find_by!(name:'JZ')
    $controller_password=kevin.password_digest
    purpose=spec[:purpose];scenario=spec[:scenario]
    actor=purpose=='link' || purpose=='sign_in' ? kevin : david
    subject='controller-member';email=purpose=='link' ? 'kevin.w@smartdata.net' : 'david@smartdata.net'
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
    before=counts
    path={'reauth'=>'/two_factor_reauthentication','sudo'=>'/sudo/google','link'=>'/user/profile/google_sign_in_link','sign_in'=>'/session/google'}.fetch(purpose)
    $controller_calls=[];$controller_transport=false;$controller_key_payload=JWKS
    client.post(path)
    start={status:client.response.status}
    query=client.response.location && Rack::Utils.parse_query(URI(client.response.location).query)
    if query && query['state']
      start[:authorize]=query.slice('scope','code_challenge_method','prompt','max_age','redirect_uri')
      if scenario=='signed_out' || scenario=='other_member'
        client.delete('/session')
        sign_in(client,jz) if scenario=='other_member'
      client.get(scenario=='other_member' ? '/users/me/profile' : '/session/new') if %w[signed_out other_member].include?(scenario)
      end
      $controller_now=BASE+601 if scenario=='expired_flow'
      auth_offset={'stale'=>-360,'before_auth_boundary'=>-331,'at_auth_boundary'=>-330,'after_auth_boundary'=>-329}.fetch(scenario,0)
      claims={'iss'=>'https://accounts.google.com','aud'=>'test-client-id','sub'=>scenario=='wrong_subject' ? 'controller-attacker' : subject,'email'=>email,'email_verified'=>true,'hd'=>scenario=='wrong_domain' ? 'wrong.test' : 'smartdata.net','name'=>'Fixture member','nonce'=>scenario=='wrong_nonce' ? 'forged' : query['nonce'],'exp'=>BASE.to_i+3600,'auth_time'=>BASE.to_i+auth_offset}
      claims.delete('auth_time') if scenario=='missing_auth'
      $controller_token_payload={'id_token'=>JWT.encode(claims,KEY,'RS256',{kid:'fixture'})}
      $controller_token_payload=spec[:payload] if scenario=='token_shape'
      $controller_key_payload=spec[:payload] if scenario=='key_shape'
      $controller_transport=scenario=='unavailable'
      call_before=$controller_calls.length
      client.get('/session/google/callback',params:{state:query['state'],code:'fixture-code',**(scenario=='cancelled' ? {error:'access_denied'} : {})})
      result=observation(client,before,call_before)
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
    end
    rows << {index:,spec:,actor_id:actor.id,jz_id:jz.id,start:,result:}
    raise ActiveRecord::Rollback
  end
end
puts JSON.pretty_generate({reference:'d7c7de92',now:BASE.to_i,rows:})
