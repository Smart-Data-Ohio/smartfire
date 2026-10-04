require "json"
require "digest"
require "cgi"
require "rack/mock"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each {|p,h|raise "Reference drift: #{p}" unless Digest::SHA256.file(Rails.root.join(p)).hexdigest==h}
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
Rails.cache=ActiveSupport::Cache::MemoryStore.new
ApplicationController.allow_forgery_protection=false
ActiveRecord::Schema.verbose=false;load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026,1,1,12)
Account.create!(name:"Connection oracle")
user=User.create!(id:811,name:"Oracle",role: :administrator)
other=User.create!(id:812,name:"Other")
bot=User.create!(id:813,name:"Machine",role: :bot)
agent=Agent.create!(id:881,user:bot,owner:user)
session=Session.create!(user:,two_factor_verified_at:Time.current)
# Replace only the external client transport, leaving validation/account/controller logic intact.
module ConnectionTransport
 def get_user
  Thread.current[:requests]<<["GET","/user",@token]
  case Thread.current[:user_status]
  when 401 then raise Github::WriteClient::Unauthorized,"Rejected"
  when 500 then raise Github::WriteClient::Error,"Transport error"
  else {"login"=>Thread.current[:login]}
  end
 end
end
Github::WriteClient.prepend(ConnectionTransport)
module AppTransport
 def exchange_code(code:,redirect_uri:)
  Thread.current[:requests]<<["POST","/login/oauth/access_token",code,redirect_uri]
  if Thread.current[:oauth_error] then raise Github::App::Unauthorized,"Rejected" end
  {"access_token"=>"fixture-app-token","refresh_token"=>"fixture-refresh","expires_in"=>28800}
 end
 def refresh_access_token(refresh_token:)
  Thread.current[:requests]<<["POST","/login/oauth/access_token",refresh_token]
  if Thread.current[:refresh_error] then raise Github::App::Error,"Transport error" end
  {"access_token"=>"fixture-fresh-token","refresh_token"=>"fixture-fresh-refresh","expires_in"=>28800}
 end
 def revoke_token(token)
  Thread.current[:requests]<<["DELETE","/applications/fixture-client/token",token];true
 end
 def revoke_grant(token)
  Thread.current[:requests]<<["DELETE","/applications/fixture-client/grant",token];true
 end
end
Github::App.singleton_class.prepend(AppTransport)
module ConnectionFlash
 def redirect_to(*args,**kwargs)
  super.tap {Thread.current[:flash]=flash.to_hash}
 end
end
[Github::ConnectionsController,Github::AppConnectionsController,Accounts::Bots::GithubConnectionsController].each{|c|c.prepend(ConnectionFlash)}
cases=[
 {name:"pat"},{name:"blank",token:"  "},{name:"rejected",user_status:401},{name:"network",user_status:500},
 {name:"profile_replace",profile:"someone-else"},{name:"release_unverified",claimant:"OctoCat"},
 {name:"verified_conflict",claimant:"octocat",claimant_verified:true},
 {name:"relink_disconnected",preset:"pat",initially_disconnected:true},
 {name:"pat_replaces_app",preset:"app"},{name:"delete_pat",method:"DELETE",preset:"pat"},
 {name:"delete_app",method:"DELETE",preset:"app"},{name:"delete_expired_app",method:"DELETE",preset:"app",expired:true},
 {name:"delete_refresh_error",method:"DELETE",preset:"app",expired:true,refresh_error:true},{name:"delete_missing",method:"DELETE"},
 {name:"bot",bot:true},{name:"bot_relink",bot:true,preset:"pat",initially_disconnected:true},
 {name:"bot_blank",bot:true,token:" "},{name:"bot_rejected",bot:true,user_status:401},{name:"bot_network",bot:true,user_status:500},{name:"bot_unlink",bot:true,method:"DELETE",preset:"pat"},
 {name:"callback",callback:true},{name:"callback_relink",callback:true,preset:"app",expired:true},
 {name:"callback_stale",callback:true,bad_state:true},{name:"callback_wrong_session",callback:true,wrong_session:true},
 {name:"callback_error",callback:true,error:"access_denied"},{name:"callback_oauth_error",callback:true,oauth_error:true},
 {name:"callback_user_rejected",callback:true,user_status:401},{name:"callback_unconfigured",callback:true,unconfigured:true}
]
vectors=cases.map do |c|
 GithubConnectedAccount.delete_all;AuditLog.delete_all
 user.update_columns(github_login:c[:profile]);other.update_columns(github_login:c[:claimant]);bot.update_columns(github_login:nil)
 ENV["GITHUB_APP_CLIENT_ID"]=c[:unconfigured] ? nil : "fixture-client";ENV["GITHUB_APP_CLIENT_SECRET"]="fixture-secret"
 target=c[:bot] ? bot : user
 if c[:preset]
  GithubConnectedAccount.create!(user:target,github_login:"old-login",access_token:"fixture-old-token",token_source:c[:preset],refresh_token:c[:preset]=="app" ? "fixture-old-refresh" : nil,token_expires_at:c[:preset]=="app" ? Time.current+(c[:expired] ? -60 : 3600) : nil,disconnected_reason:c[:initially_disconnected] ? "Disconnected" : nil)
 end
 GithubConnectedAccount.create!(user:other,github_login:"octocat",access_token:"fixture-other") if c[:claimant_verified]
 request=ActionDispatch::Request.new(Rails.application.env_config.merge(Rack::MockRequest.env_for("http://example.org/")))
 cookies=ActionDispatch::Cookies::CookieJar.build(request,{})
 cookies.signed[:session_token]=session.token
 cookies.encrypted[:_campfire_session]={value:{"session_id"=>"0123456789abcdef0123456789abcdef","sudo_verified_at"=>Time.current.to_i,"github_app_oauth_state"=>c[:wrong_session] ? "wrong" : "fixture-state"}}
 cookie="session_token=#{CGI.escape(cookies[:session_token])}; _campfire_session=#{CGI.escape(cookies[:_campfire_session])}"
 Thread.current[:requests]=[];Thread.current[:flash]={};Thread.current[:user_status]=c.fetch(:user_status,200);Thread.current[:login]="octocat";Thread.current[:oauth_error]=c[:oauth_error];Thread.current[:refresh_error]=c[:refresh_error]
 body={access_token:c.fetch(:token,"fixture-pasted")};method=c.fetch(:method,"POST");path=c[:bot] ? "/account/bots/813/github_connection" : "/github/connection"
 if c[:callback]
  method="GET";signed=c[:bad_state] ? "bogus" : Rails.application.message_verifier("github_app_oauth_state").generate("fixture-state")
  path="/github/app/callback?"+URI.encode_www_form(code:"fixture-code",state:signed,error:c[:error])
 end
 res=Rack::MockRequest.new(Rails.application).request(method,"http://example.org"+path,"HTTP_COOKIE"=>cookie,"CONTENT_TYPE"=>"application/json",input:JSON.generate(body))
 a=target.reload.github_connected_account
 account=a && a.attributes.slice("github_login","token_source","disconnected_reason","last_error").merge("access_token"=>a.access_token,"refresh_token"=>a.refresh_token,"token_expires_at"=>a.token_expires_at&.iso8601)
 audit=AuditLog.order(:id).map{|l|l.attributes.slice("action","actor_id","target_type","target_id","details")}
 {**c,request_body:body,status:res.status,location:res["Location"],flash:Thread.current[:flash],account:,profile:user.reload.github_login,claimant_login:other.reload.github_login,requests:Thread.current[:requests],audit:}
end
File.write("/work/vectors/github_connections_http.json",JSON.pretty_generate(vectors)+"\n")
puts "GitHub connections Rails oracle: #{vectors.size} HTTP/status/flash/identity/revocation cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
