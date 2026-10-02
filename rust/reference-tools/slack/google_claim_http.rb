# WS16 / WS14g owner handoff: real callbacks, sessions, CSRF and personal opt-in.
# Only outbound providers, clock, rendering entropy and OAuth entropy are fixtures.
require 'json'
require 'openssl'
require 'digest'
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
Rails.logger=ActiveSupport::Logger.new($stderr)
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
Rails.cache=ActiveSupport::Cache::MemoryStore.new
ApplicationController.allow_forgery_protection=true
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_) { 'NONCE' }
ApplicationController.prepend(Module.new do
  def form_authenticity_token(form_options:{})
    super # Store a real session secret; fixture display tokens never authorize writes.
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
  def content_security_policy_nonce; 'NONCE'; end
end)
ENV['VAPID_PUBLIC_KEY']='';Rails.configuration.x.vapid.public_key=nil
ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-google-client-secret'
ENV['GOOGLE_SIGN_IN_DOMAINS']='smartdata.net'
WORK=ENV.fetch('PARITY_WORK')
INPUT=JSON.parse(File.read(File.join(WORK,'crates/campfire/src/app/google_tests/slack_claim.json')))
KEY=OpenSSL::PKey.read(File.binread(File.join(WORK,'crates/campfire/src/integrations/google/signing.der')))
JWKS=JSON.parse(File.read(File.join(WORK,'crates/campfire/src/integrations/google/test-jwks.json')))
GRANT=['fixture','claim','grant'].join('-')
AUTH=JSON.parse(File.read(File.join(WORK,'vectors/two_factor_views.json')))
TwoFactorCredential.define_singleton_method(:generate_secret) { 'JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP' }
backup_index=0
TwoFactorBackupCode.define_singleton_method(:generate_code) do
  code=AUTH.fetch('codes').fetch(backup_index);backup_index+=1;code
end
$calls=[];$id_token=nil
Google::SignIn::KeyStore.clear!
Google::Client.define_singleton_method(:post_token_form) do |**form|
  raise 'bad Google exchange' unless form[:grant_type]=='authorization_code' && form[:code_verifier].present? && form[:redirect_uri]=='http://campfire.test/session/google/callback'
  $calls << {'provider'=>'google','path'=>'/token','form'=>form.slice(:code,:redirect_uri,:grant_type,:code_verifier).stringify_keys}
  response=Net::HTTPOK.new('1.1','200','OK');response.instance_variable_set(:@read,true)
  response.body=JSON.generate({'id_token'=>$id_token});response
end
# Scoped fixture entropy, with real verifiers/PKCE/signatures, matches Rust's cfg(test) seam.
SecureRandom.singleton_class.prepend(Module.new do
  def hex(n=nil)
    Thread.current[:ws16_hex]&.shift || super
  end
end)
Google::SignIn.singleton_class.prepend(Module.new do
  def pkce_pair
    return super unless (byte=Thread.current[:ws16_verifier])
    verifier=Base64.urlsafe_encode64([byte].pack('C')*32,padding:false)
    [verifier,Base64.urlsafe_encode64(Digest::SHA256.digest(verifier),padding:false)]
  end
end)
SlackImport::Runner.step_budget=0.seconds
Slack::Client.prepend(Module.new do
  def initialize(**args);super(**args.merge(pacing:false));end
end)
transport=Object.new
transport.define_singleton_method(:request) do |request|
  uri=URI(request.path);query=URI.decode_www_form(uri.query.to_s).to_h
  raise 'bad Slack authorization' unless request['Authorization']==['Bearer',GRANT].join(' ')
  data=case uri.path
  when '/api/users.list' then {'ok'=>true,'members'=>INPUT['members']}
  when '/api/conversations.list' then {'ok'=>true,'channels'=>[INPUT['conversation']]}
  when '/api/conversations.members' then {'ok'=>true,'members'=>%w[UCLAIM UPEER]}
  when '/api/conversations.history' then {'ok'=>true,'messages'=>INPUT['history'],'has_more'=>false}
  when '/api/conversations.replies' then {'ok'=>true,'messages'=>INPUT['replies'],'has_more'=>false}
  else raise "unrecorded Slack path #{uri.path}"
  end
  $calls << {'provider'=>'slack','path'=>uri.path,'query'=>query}
  response=Net::HTTPOK.new('1.1','200','OK');response.instance_variable_set(:@read,true);response.body=JSON.generate(data);response
end
transport.define_singleton_method(:get) {|path,headers| request(Net::HTTP::Get.new(path,headers))}
transport.define_singleton_method(:post) do |path,body,headers|
  raise 'unrecorded Slack exchange' unless path=='/api/oauth.v2.access'
  form=URI.decode_www_form(body).to_h
  $calls << {'provider'=>'slack','path'=>path,'form'=>form.except('client_secret')}
  data={'ok'=>true,'team'=>{'id'=>'TCLAIM','name'=>'Claim fixture'},'authed_user'=>{'id'=>'UCLAIM','access_token'=>GRANT,'scope'=>Slack::OAuth::USER_SCOPES.join(',')}}
  response=Net::HTTPOK.new('1.1','200','OK');response.instance_variable_set(:@read,true);response.body=JSON.generate(data);response
end
Net::HTTP.define_singleton_method(:start) do |host,*args,**options,&block|
  if host=='slack.com'
    block.call(transport)
  elsif host=='www.googleapis.com'
    http=Object.new
    http.define_singleton_method(:get) do |path|
      raise 'unrecorded Google path' unless path=='/oauth2/v3/certs'
      $calls << {'provider'=>'google','path'=>path}
      response=Net::HTTPOK.new('1.1','200','OK');response.instance_variable_set(:@read,true);response.body=JSON.generate(JWKS);response
    end
    block.call(http)
  else
    raise "unrecorded external host #{host}"
  end
end
# Existing admin import creates the placeholder and historical ownership through real mappers.
GoogleIdentity.delete_all;AuditLog.delete_all
admin=User.find(127326141)
db=ActiveRecord::Base.connection
%w[users rooms messages channel_threads memberships thread_memberships slack_imports slack_import_records slack_workspaces slack_connections google_identities].each do |table|
  db.execute("DELETE FROM sqlite_sequence WHERE name='#{table}'")
  db.execute("INSERT INTO sqlite_sequence(name,seq) VALUES('#{table}',9000000000)")
end
workspace=SlackWorkspace.create!(client_id:'fixture-client',client_secret:'fixture-secret',configured_by:admin,team_id:'TCLAIM',team_name:'Claim fixture')
run=SlackImport.create!(slack_workspace:workspace,user:admin,kind:'workspace',mode:'import',status:'completed')
mapper=Slack::UserMapper.new(workspace:,run:)
mapper.map_page(INPUT['members'])
users=mapper.users_for(%w[UCLAIM UPEER])
room=Slack::ConversationMapper.new(workspace:,run:).resolve(INPUT['initial_conversation'],member_ids:%w[UCLAIM UPEER],users:).room
writer=Slack::MessageWriter.new(workspace:,run:,user_mapper:mapper)
result=writer.write_history_page(room:,conversation_id:'CCLAIM',messages:INPUT['initial_history'],bounds:{},users:)
writer.write_replies_page(room:,conversation_id:'CCLAIM',parent_ts:'1700000000.000001',parent_message_id:result[:thread_parents].first['message_id'],messages:INPUT['initial_replies'],bounds:{},users:,direct:false,thread_state:{})
claimant=users.fetch('UCLAIM')
raise 'not a claimable placeholder' unless claimant.google_email_link_allowed? && claimant.password_digest.nil? && claimant.google_identity.nil?
COLUMNS={
 'users'=>%w[id name email_address role status time_zone bio google_email_link_allowed email_self_changed_at],
 'memberships'=>%w[id room_id user_id last_read_message_id unread_at],
 'messages'=>%w[id room_id creator_id thread_id reply_to_message_id markdown_source],
 'channel_threads'=>%w[id room_id creator_id parent_message_id name],
 'thread_memberships'=>%w[id thread_id user_id],
 'slack_import_records'=>%w[id slack_workspace_id slack_import_id slack_kind slack_key record_type record_id created_record],
 'slack_imports'=>%w[id slack_workspace_id slack_connection_id user_id kind mode status options],
 'google_identities'=>%w[user_id subject email domain],
 'audit_logs'=>%w[action actor_id target_id target_type target_label details]
}
def snapshot(claimant,workspace)
 db=ActiveRecord::Base.connection;db.clear_query_cache
 state=COLUMNS.to_h do |table,columns|
  predicate=case table
  when 'users' then "WHERE id=#{claimant.id}"
  when 'memberships' then "WHERE user_id=#{claimant.id} OR room_id IN (SELECT record_id FROM slack_import_records WHERE record_type='Room')"
  when 'messages','channel_threads' then 'WHERE id>9000000000'
  when 'thread_memberships' then 'WHERE thread_id>9000000000'
  else ''
  end
  rows=db.select_all("SELECT #{columns.map { |c|db.quote_column_name(c) }.join(',')} FROM #{db.quote_table_name(table)} #{predicate} ORDER BY #{table=='google_identities' ? 'user_id' : 'id'}").to_a
  %w[options details].each { |c| rows.each { |r|r[c]=JSON.parse(r[c]) if r[c].is_a?(String) } }
  [table,rows]
 end
 state['sessions']=db.select_all("SELECT user_id,two_factor_verified_at,last_active_at FROM sessions WHERE user_id=#{claimant.id} ORDER BY id").to_a
 state['google_accounts']=GoogleAccount.where(user_id:claimant.id).count
 connection=SlackConnection.find_by(user_id:claimant.id)
 state['connection']=connection&.attributes&.slice('id','slack_workspace_id','user_id','slack_user_id','scopes','disconnected_reason')
 state['credential_checks']={password_absent:claimant.reload.password_digest.nil?,token_matches:connection ? connection.access_token==GRANT : nil,encrypted:connection ? db.select_value("SELECT access_token FROM slack_connections WHERE id=#{connection.id}")!=GRANT : nil,enrolled:claimant.two_factor_enabled?,backup_count:TwoFactorBackupCode.joins(:two_factor_credential).where(two_factor_credentials:{user_id:claimant.id}).count}
 state
end
client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
rows=[];initial=snapshot(claimant,workspace)
request=lambda do |name,method,path,params={},bad_csrf:false,entropy:nil|
 if entropy
  Thread.current[:ws16_hex]=entropy.first(2).map {|b|[b].pack('C').unpack1('H*')*16 }
  Thread.current[:ws16_verifier]=entropy[2]
 end
 headers={'HTTP_ACCEPT'=>'text/html'}
 headers['HTTP_X_CSRF_TOKEN']=bad_csrf ? 'invalid' : client.request.session[:_csrf_token] if method!='get'
 client.public_send(method,path,params:,headers:)
 rows << {name:,status:client.response.status,location:client.response.location,content_type:client.response.headers['Content-Type'],body:client.response.body,flash:client.response.redirect? ? client.request.flash.to_hash : {},state:snapshot(claimant,workspace)}
 Thread.current[:ws16_hex]=nil;Thread.current[:ws16_verifier]=nil
 client.response
end
issue_token=lambda do |query|
 claims={'iss'=>'https://accounts.google.com','aud'=>'test-client-id','sub'=>'ws16-claim-subject','email'=>'jane@smartdata.net','email_verified'=>true,'hd'=>'smartdata.net','name'=>'Google Jane','nonce'=>query.fetch('nonce'),'exp'=>Time.current.to_i+3600,'auth_time'=>Time.current.to_i}
 $id_token=JWT.encode(claims,KEY,'RS256',{kid:'fixture'})
end
drive=lambda do |id|
 100.times do
  Rails.application.executor.run!(reset:true)
  SlackImport::StepJob.perform_now(SlackImport.find(id))
  return if SlackImport.find(id).completed?
 end
 raise 'recorded personal job did not finish'
end
request.call('login','get','/session/new')
request.call('google-csrf','post','/session/google',{},bad_csrf:true)
request.call('google-start','post','/session/google',{},entropy:[1,2,3])
q=Rack::Utils.parse_query(URI(client.response.location).query);issue_token.call(q)
request.call('google-claim','get','/session/google/callback', {state:q.fetch('state'),code:'fixture-code'})
request.call('enrollment-required','get','/slack/imports')
request.call('enrollment-setup','get','/two_factor_setup')
setup=TwoFactorSetupSecret.order(:id).last
code=ROTP::TOTP.new(setup.secret).at(Time.current)
request.call('enrollment-csrf','post','/two_factor_setup',{code:},bad_csrf:true)
request.call('enrollment-confirm','post','/two_factor_setup',{code:})
request.call('personal-unconnected','get','/slack/imports')
request.call('preview-needs-opt-in','post','/slack/imports')
request.call('opt-in-alert','get','/slack/imports')
request.call('slack-needs-sudo','get','/slack/oauth/start?return_to=/slack/imports')
request.call('sudo-prompt','get','/sudo/new')
request.call('sudo-csrf','post','/sudo/google',{},bad_csrf:true)
request.call('google-sudo-start','post','/sudo/google',{},entropy:[4,5,6])
q=Rack::Utils.parse_query(URI(client.response.location).query);issue_token.call(q)
request.call('google-sudo-confirm','get','/session/google/callback',{state:q.fetch('state'),code:'fixture-code'})
request.call('slack-start','get','/slack/oauth/start?return_to=/slack/imports',{},entropy:[7])
request.call('slack-forged-state','get','/slack/oauth/callback',{state:'forged',code:'fixture-slack-code'})
request.call('slack-expired-page','get','/slack/imports')
request.call('slack-retry','get','/slack/oauth/start?return_to=/slack/imports',{},entropy:[7])
q=Rack::Utils.parse_query(URI(client.response.location).query)
request.call('slack-opt-in','get','/slack/oauth/callback',{state:q.fetch('state'),code:'fixture-slack-code'})
request.call('personal-connected','get','/slack/imports')
request.call('preview-csrf','post','/slack/imports',{},bad_csrf:true)
request.call('preview-start','post','/slack/imports')
preview=SlackImport.order(:id).last;drive.call(preview.id)
request.call('preview-complete','get',"/slack/imports/#{preview.id}")
request.call('import-start','post','/slack/imports',{mode:'import',dry_run_id:preview.id,conversation_ids:['DCLAIM']})
personal=SlackImport.order(:id).last;drive.call(personal.id)
request.call('import-complete','get',"/slack/imports/#{personal.id}")
request.call('personal-final','get','/slack/imports')
request.call('slack-replay','get','/slack/oauth/callback',{state:q.fetch('state'),code:'fixture-slack-code'})
request.call('replay-page','get','/slack/imports')
sources=%w[app/controllers/sessions/google_controller.rb app/controllers/concerns/google_sign_in_flow.rb app/controllers/concerns/two_factor_enforcement.rb app/controllers/two_factor/setups_controller.rb app/controllers/sudos_controller.rb app/controllers/slack/oauth_controller.rb app/controllers/slack/imports_controller.rb app/models/google/sign_in/account_linker.rb app/models/slack/user_mapper.rb app/models/slack/message_writer.rb]
raise 'claim lost historical identity' unless GoogleIdentity.find_by!(subject:'ws16-claim-subject').user_id==claimant.id && User.where("LOWER(email_address) = ?",'jane@smartdata.net').count==1
raise 'personal import failed' unless personal.reload.completed? && preview.reload.completed? && SlackConnection.find_by!(user_id:claimant.id).slack_user_id=='UCLAIM'
output={reference:'d7c7de92',layout_reference:'2e20b24c',sources:sources.to_h {|p|[p,Digest::SHA256.file(Rails.root.join(p)).hexdigest]},columns:COLUMNS,initial:,rows:,calls:$calls}
File.write(File.join(WORK,'vectors/slack/google_claim_http.json'),JSON.pretty_generate(output)+"\n")
puts "Google → Slack claim Rails oracle: #{rows.size} HTTP responses; 2 Google verifications; personal preview/import completed; #{COLUMNS.size} ownership tables per stage"
