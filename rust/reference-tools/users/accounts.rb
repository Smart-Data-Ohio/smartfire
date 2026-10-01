require "json"
require "digest"
Rails.application.config.hosts.clear
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/accounts-source-hashes.json"))).each do |file,expected|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==expected
end
user=User.find(127326141)
target=User.find(712064548)
account=Account.first
browser=ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax}
browser.cookies["session_token"]=request.cookie_jar[:session_token]
browser.get "/account/edit"
ActiveSupport::IsolatedExecutionState.clear
token=Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')["content"]
headers={"X-CSRF-Token"=>token,"Accept"=>"text/html","User-Agent"=>"ws8br2-account-fixture"}
browser.post "/sudo",params:{password:"secret123456"},headers:headers
raise "sudo did not confirm: #{browser.response.status}" unless browser.response.redirect?
ActiveSupport::IsolatedExecutionState.clear
cases=[
 ["settings_name","put","/account",{account:{name:"Different"}}],
 ["settings_noop","put","/account",{account:{name:"Signal"}}],
 ["settings_restrict","put","/account",{account:{settings:{restrict_room_creation_to_administrators:"1"}}}],
 ["settings_unknown","put","/account",{account:{name:"must not save",settings:{unknown:"1"}}}],
 ["custom_styles","put","/account/custom_styles",{account:{custom_styles:"/* 🌴 */ :root { color: red; }"}}],
 ["styles_noop","put","/account/custom_styles",{account:{custom_styles:"old"}},"old"],
 ["styles_nil","put","/account/custom_styles",{account:{custom_styles:nil}},"old"],
 ["styles_unpermitted","put","/account/custom_styles",{account:{custom_styles:["wrong"]}},"old"],
 ["logo_delete","delete","/account/logo",{}],
 ["join_reset","post","/account/join_code",{}],
 ["role_promote","put","/account/users/712064548",{user:{role:"administrator"}}],
 ["role_noop","put","/account/users/712064548",{user:{role:"member"}}],
 ["role_invalid","put","/account/users/712064548",{user:{role:"bot"}}],
 ["role_failed_validation","put","/account/users/712064548",{user:{role:"administrator"}},nil,"neon"],
 ["role_self_noop","put","/account/users/127326141",{user:{role:"administrator"}}],
 ["ban_private","post","/users/712064548/ban",{},nil,nil,0,"127.0.0.1"],
 ["ban_active","post","/users/712064548/ban",{},nil,nil,0,"203.0.113.42"],
 ["ban_deactivated","post","/users/712064548/ban",{},nil,nil,1],
 ["ban_replay","post","/users/712064548/ban",{},nil,nil,2],
 ["unban_banned","delete","/users/712064548/ban",{},nil,nil,2],
 ["unban_active","delete","/users/712064548/ban",{},nil,nil,0],
 ["unban_deactivated","delete","/users/712064548/ban",{},nil,nil,1],
 ["deactivate","delete","/account/users/712064548",{}],
 ["deactivate_self","delete","/account/users/127326141",{}]
].map do |name,method,path,params,styles,theme,status,session_ip|
  account.update_columns(name:"Signal",custom_styles:styles,settings:{restrict_room_creation_to_administrators:false})
  target.update_columns(role:0,status:status || 0,theme:theme || "system")
  target.sessions.update_all(ip_address:session_ip) if session_ip
  target.sessions.create!(ip_address:"203.0.113.43",user_agent:"ws8br2-target") if name=="ban_active"
  AuditLog.delete_all
  old_code=account.reload.join_code
  active=User.active.count
  browser.public_send(method,path,params:params,as: :json,headers:headers)
  ActiveSupport::IsolatedExecutionState.clear
  subject_id=path.match(%r{/(?:account/)?users/(\d+)})&.captures&.first
  subject=User.find(subject_id) if subject_id
  state={account_name:account.reload.name,restrict:account.settings.restrict_room_creation_to_administrators?,styles:account.custom_styles,code_changed:account.join_code!=old_code,role:target.reload.role,status:target.status,subject_status:subject&.status,subject_role:subject&.role,active_delta:User.active.count-active,target_sessions:target.sessions.count,banned_ips:target.bans.order(:id).pluck(:ip_address)}
  rows=AuditLog.order(:id).map{|row|row.attributes.slice("action","actor_id","actor_label","target_type","target_id","target_label","details","ip_address","user_agent")}
  {name:name,method:method.upcase,path:path,params:params,styles_before:styles,theme_before:theme || "system",status_before:status || 0,session_ip:session_ip,status:browser.response.status,location:browser.response.location,state:state,audits:rows}
end
deferred,cases=cases.partition{|c|c[:name]=="deactivate_self"}
puts JSON.pretty_generate(reference:"d7c7de92",cases:cases,deferred_agent_owner_cases:deferred)
warn "Rails account mutation oracle: #{cases.size} HTTP cases with audit snapshots, #{deferred.size} deferred agent-owner case; reference d7c7de92"
