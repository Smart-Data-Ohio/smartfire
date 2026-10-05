# Each original Accounts and Bans assertion, observed after routed Rails writes.
require 'json'
Rails.application.config.hosts.clear
Rails.logger=ActiveSupport::Logger.new($stderr)
ActionView::Base.logger=Rails.logger
cases=[
 ['role_self',127326141,'PUT','/account/users/127326141',{user:{role:'administrator'}},%w[admin_before admin_after]],
 ['remove_self',127326141,'DELETE','/account/users/127326141',{},%w[active_delta david_active]],
 ['member_role',712064548,'PUT','/account/users/127326141',{user:{role:'administrator'}},[]],
 ['member_remove',712064548,'DELETE','/account/users/127326141',{},[]],
 ['styles_edit',127326141,'GET','/account/custom_styles/edit',{},[]],
 ['styles',127326141,'PUT','/account/custom_styles',{account:{custom_styles:':root { --color-text: red; }'}},%w[admin_before styles]],
 ['member_styles',712064548,'PUT','/account/custom_styles',{account:{custom_styles:':root { --color-text: red; }'}},%w[member_before]],
 ['join',127326141,'POST','/account/join_code',{},%w[code_changed]],
 ['member_join',773523953,'POST','/account/join_code',{},[]],
 ['ban_two',127326141,'POST','/users/712064548/ban',{},%w[ban_delta ips],%w[203.0.113.1 203.0.113.2]],
 ['ban_session',127326141,'POST','/users/712064548/ban',{},%w[session_delta],%w[203.0.113.1]],
 ['unban',127326141,'DELETE','/users/712064548/ban',{},%w[banned_before bans_before ban_delta kevin_active],%w[203.0.113.1]],
 ['member_ban',712064548,'POST','/users/773523953/ban',{},[]],
 ['member_unban',712064548,'DELETE','/users/773523953/ban',{},[]]
].map do |name,viewer,method,path,params,keys,ips|
 row=nil
 ActiveRecord::Base.transaction(requires_new:true) do
  admin=User.find(127326141);member=User.find(712064548);account=Account.first
  if name.start_with?('ban_') || name=='unban'
   member.sessions.destroy_all;member.bans.destroy_all
   (ips||[]).each{|ip|member.sessions.create!(ip_address:ip,user_agent:'Test')}
   member.ban if name=='unban'
  end
  User.find(773523953).banned! if name=='member_unban'
  before={admin:admin.administrator?,member:member.member?,active:User.active.count,ban_total:Ban.count,bans:member.bans.count,sessions:member.sessions.count,code:account.join_code,banned:member.reload.banned?}
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
  request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  request.cookie_jar.signed.permanent[:session_token]={value:User.find(viewer).sessions.first.token,httponly:true,same_site: :lax}
  browser.cookies['session_token']=request.cookie_jar[:session_token]
  browser.get '/users/me/profile';ActiveSupport::IsolatedExecutionState.clear
  csrf=Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
  headers={'X-CSRF-Token'=>csrf}
  if viewer==127326141
   browser.post '/sudo',params:{password:'secret123456'},headers:headers;ActiveSupport::IsolatedExecutionState.clear
   raise 'sudo' unless browser.response.status==302
  end
  if method=='GET' then browser.get path else browser.public_send(method.downcase,path,params:params,as: :json,headers:headers) end
  ActiveSupport::IsolatedExecutionState.clear
  state={admin_before:before[:admin],member_before:before[:member],admin_after:admin.reload.administrator?,active_delta:User.active.count-before[:active],david_active:User.active.exists?(127326141),styles:account.reload.custom_styles,code_changed:account.join_code!=before[:code],ban_delta:Ban.count-before[:ban_total],ips:member.bans.order(:ip_address).pluck(:ip_address),session_delta:member.sessions.count-before[:sessions],banned_before:before[:banned],bans_before:before[:bans],kevin_active:member.reload.active?}
  response={status:browser.response.status,location:browser.response.location}.merge(state.slice(*keys.map(&:to_sym)))
  row={name:name,viewer:viewer,method:method,path:path,params:params,ips:ips||[],response:response}
  raise ActiveRecord::Rollback
 end
 row
end
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),cases:cases)
warn "Rails original account oracle: #{cases.size} routed response/state cases"
