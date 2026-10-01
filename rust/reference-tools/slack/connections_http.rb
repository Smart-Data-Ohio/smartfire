require 'json'
require 'cgi'
require 'rack/mock'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
Rails.cache = ActiveSupport::Cache::MemoryStore.new
ApplicationController.allow_forgery_protection = false
ActiveRecord::Schema.verbose = false
load Rails.root.join('db/schema.rb')
travel_to Time.utc(2026,1,1,12)
Account.create!(name:'Slack oracle')
user=User.create!(id:811,name:'Oracle',role: :administrator)
other=User.create!(id:812,name:'Other')
session=Session.create!(user:,two_factor_verified_at:Time.current)
transport=Object.new
transport.define_singleton_method(:post) do |path,body,headers|
 Thread.current[:requests] << {method:'POST',path:,form:URI.decode_www_form(body).to_h}
 data=path=='/api/auth.revoke' ? {'revoked'=>true} : Thread.current[:exchange]
 response=Net::HTTPOK.new('1.1','200','Fixture');response.instance_variable_set(:@read,true);response.body=JSON.generate(data);response
end
transport.define_singleton_method(:request) do |req|
 raise 'Bad authorization' unless req['Authorization']==['Bearer','fixture-user-grant'].join(' ')
 Thread.current[:requests] << {method:'GET',path:req.path}
 data=Thread.current[:team]
 response=Net::HTTPOK.new('1.1','200','Fixture');response.instance_variable_set(:@read,true);response.body=JSON.generate(data);response
end
Net::HTTP.singleton_class.prepend(Module.new do
 define_method(:start) do |host,port,**opts,&block|
  raise 'Unexpected external endpoint' unless host=='slack.com' && port==443
  block.call(transport)
 end
end)
flash_capture=Module.new do
 def redirect_to(*args,**kwargs)
  super.tap {Thread.current[:flash]=flash.to_hash}
 end
end
[Slack::OAuthController,Slack::ConnectionsController,Accounts::SlackImportsController].each{|c|c.prepend(flash_capture)}
cases=[
 {name:'callback'}, {name:'callback_member',role:0}, {name:'callback_known',team_id:'TFIXTURE'},
 {name:'callback_reconnect',preset:true,disconnected:true,team_id:'TFIXTURE'},
 {name:'callback_unreadable_reconnect',preset:true,unreadable:true,team_id:'TFIXTURE'},
 {name:'callback_invalid_team_info',invalid_team_info:true},
 {name:'callback_team_info_absent',team_missing:true},{name:'callback_team_name_fallback',blank_team_name:true},
 {name:'callback_wrong_team',team_id:'TOTHER',team_name:'Other'}, {name:'callback_missing_scopes',scopes:'channels:history,channels:read'},
 {name:'callback_claimed',claimed:true}, {name:'callback_invalid_code',exchange_error:true},
 {name:'callback_denied',error:'access_denied'},{name:'callback_tampered',bad_state:true},
 {name:'callback_wrong_user',state_user:812},{name:'callback_wrong_session',wrong_state:true},
 {name:'callback_removed',unconfigured:true},{name:'callback_personal_return',return_to:'/slack/imports'},
 {name:'callback_evil_return',return_to:'https://evil.test/phish'},
 {name:'disconnect',method:'DELETE',path:'/slack/connection',preset:true},
 {name:'disconnect_member',method:'DELETE',path:'/slack/connection',preset:true,role:0},
 {name:'disconnect_personal_return',method:'DELETE',path:'/slack/connection',preset:true,return_to:'/slack/imports'},
 {name:'disconnect_missing',method:'DELETE',path:'/slack/connection'},
 {name:'disconnect_unreadable',method:'DELETE',path:'/slack/connection',preset:true,unreadable:true},
 {name:'disconnect_active',method:'DELETE',path:'/slack/connection',preset:true,active:true},
 {name:'disconnect_other_active',method:'DELETE',path:'/slack/connection',preset:true,other_active:true},
 {name:'disconnect_sudo',method:'DELETE',path:'/slack/connection',preset:true,no_sudo:true},
 {name:'setup_save',method:'PATCH',path:'/account/slack_import',unconfigured:true,body:{client_id:' client ',client_secret:' secret '}},
 {name:'setup_keep_secret',method:'PATCH',path:'/account/slack_import',body:{client_id:'new-client',client_secret:'  '}},
 {name:'setup_invalid',method:'PATCH',path:'/account/slack_import',unconfigured:true,body:{client_id:' ',client_secret:''}},
 {name:'setup_sudo',method:'PATCH',path:'/account/slack_import',no_sudo:true,body:{client_id:'new-client',client_secret:'new-secret'}},
 {name:'setup_forbidden',method:'PATCH',path:'/account/slack_import',role:0,body:{client_id:'new-client',client_secret:'new-secret'}},
 {name:'remove',method:'DELETE',path:'/account/slack_import',preset:true,history:true},
 {name:'remove_active',method:'DELETE',path:'/account/slack_import',preset:true,active:true},
 {name:'remove_sudo',method:'DELETE',path:'/account/slack_import',preset:true,no_sudo:true},
 {name:'remove_forbidden',method:'DELETE',path:'/account/slack_import',preset:true,role:0},
 {name:'remove_missing',method:'DELETE',path:'/account/slack_import',unconfigured:true}
]
vectors=cases.map do |c|
 ActiveRecord::Base.clear_query_caches_for_current_thread
 load Rails.root.join('db/schema.rb')
 Account.create!(name:'Slack oracle')
 user=User.create!(id:811,name:'Oracle',role: :administrator)
 other=User.create!(id:812,name:'Other')
 session=Session.create!(user:,two_factor_verified_at:Time.current)
 SlackImport::Issue.delete_all;SlackImport::Record.delete_all;SlackImport.delete_all;SlackConnection.delete_all;SlackWorkspace.delete_all;AuditLog.delete_all
 ActiveRecord::Base.connection.execute("DELETE FROM sqlite_sequence WHERE name IN ('slack_workspaces','slack_connections','slack_imports')")
 user.update_columns(role:c.fetch(:role,1))
 workspace=SlackWorkspace.create!(id:851,client_id:'fixture-client',client_secret:'fixture-secret',configured_by:user,team_id:c[:team_id],team_name:c[:team_name]) unless c[:unconfigured]
 if c[:preset]
  connection=SlackConnection.create!(id:852,slack_workspace:workspace,user:,slack_user_id:'UFIXTURE',access_token:'fixture-user-grant',scopes:Slack::OAuth::USER_SCOPES.join(','),disconnected_reason:c[:disconnected] ? 'Rejected' : nil)
  ActiveRecord::Base.connection.execute("UPDATE slack_connections SET access_token='unreadable-fixture' WHERE id=#{connection.id}") if c[:unreadable]
 end
 SlackConnection.create!(slack_workspace:workspace,user:other,slack_user_id:'UFIXTURE',access_token:'fixture-other') if c[:claimed]
 if c[:active] || c[:other_active] || c[:history]
  SlackImport.create!(id:853,slack_workspace:workspace,user:c[:other_active] ? other : user,slack_connection:connection,kind:'personal',mode:'import',status:c[:history] ? 'completed' : 'running')
 end
 Thread.current[:requests]=[];Thread.current[:flash]={}
 Thread.current[:exchange]={'ok'=>true,'team'=>{'id'=>'TFIXTURE','name'=>c[:blank_team_name] ? nil : 'Fixture'},'authed_user'=>{'id'=>'UFIXTURE','access_token'=>'fixture-user-grant','scope'=>c.fetch(:scopes,Slack::OAuth::USER_SCOPES.join(','))}}
 Thread.current[:exchange]={'ok'=>false,'error'=>'invalid_code'} if c[:exchange_error]
 Thread.current[:team]=c[:team_missing] ? {} : {'ok'=>true,'team'=> c[:invalid_team_info] ? 42 : {'id'=>'TFIXTURE','name'=>'Team fallback','domain'=>'fixture'}}
 request=ActionDispatch::Request.new(Rails.application.env_config.merge(Rack::MockRequest.env_for('http://example.org/')))
 cookies=ActionDispatch::Cookies::CookieJar.build(request,{})
 cookies.signed[:session_token]=session.token
 values={'session_id'=>'0123456789abcdef0123456789abcdef','slack_oauth_state'=>{'state'=>c[:wrong_state] ? 'wrong' : 'fixture-state','user_id'=>c.fetch(:state_user,811)},'slack_oauth_return_to'=>c[:return_to]}
 values['sudo_verified_at']=Time.current.to_i unless c[:no_sudo]
 cookies.encrypted[:_campfire_session]={value:values}
 cookie="session_token=#{CGI.escape(cookies[:session_token])}; _campfire_session=#{CGI.escape(cookies[:_campfire_session])}"
 path=c.fetch(:path,'/slack/oauth/callback');method=c.fetch(:method,'GET');body=c.fetch(:body,{})
 if method=='GET'
  signed=c[:bad_state] ? 'bogus' : Rails.application.message_verifier('slack_oauth_state').generate('fixture-state')
  path+='?'+URI.encode_www_form(code:'fixture-code',state:signed,error:c[:error])
 elsif c[:return_to]
  body=body.merge(return_to:c[:return_to])
 end
 ActiveRecord::Base.clear_query_caches_for_current_thread
 sequences=ActiveRecord::Base.connection.select_all("SELECT name,seq FROM sqlite_sequence WHERE name IN ('slack_workspaces','slack_connections','slack_imports')").to_a
 res=Rack::MockRequest.new(Rails.application).request(method,'http://example.org'+path,'HTTP_COOKIE'=>cookie,'CONTENT_TYPE'=>'application/json',input:JSON.generate(body))
 connection=user.reload.slack_connection
 conn=connection && connection.slice('slack_workspace_id','user_id','slack_user_id','scopes','disconnected_reason').merge('access_token'=> (connection.access_token rescue nil))
 ws=SlackWorkspace.current
 w=ws && ws.attributes.slice('client_id','configured_by_id','team_id','team_name','team_domain').merge('client_secret'=>ws.client_secret)
 audit=AuditLog.order(:id).map{|l|l.attributes.slice('action','actor_id','target_type','target_id','details')}
 {**c,sequences:,status:res.status,location:res['Location'],flash:Thread.current[:flash],connection:conn,workspace:w,requests:Thread.current[:requests],audit:,history_connection_id:SlackImport.find_by(id:853)&.slack_connection_id}
end
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/connections_http.json'),JSON.pretty_generate(vectors)+"\n")
puts "Slack connection HTTP oracle: #{vectors.size} real Rails callback/disconnect/setup/remove cases generated"
