# Original AuditLog::RoomsAuditTest requests and audit assertions at d7c7de92.
require 'json'
Rails.application.config.hosts.clear
Rails.logger=ActiveSupport::Logger.new($stderr);ActionView::Base.logger=Rails.logger
specs=[
 ['create','room.create',nil,[['POST','/rooms/opens',{room:{name:'Audited Room'}}]]],
 ['failed','room.create',nil,[['POST','/rooms/closeds',{room:{name:'Iconic',icon_name:':notanicon:'},user_ids:[127326141]}]]],
 ['direct_reuse','room.create',nil,Array.new(2){['POST','/rooms/directs',{user_ids:[127326141,149087659,712064548]}]}],
 ['destroy','room.destroy',{type:'closed',name:'Doomed',members:[127326141]},[['DELETE','/rooms/ROOM',{}]]],
 ['membership','room.membership.change',{designers:true,replace:true},[['PUT','/rooms/closeds/654632876',{room:{name:'Designers'},user_ids:'REVISED'}]]],
 ['unchanged','room.membership.change',{designers:true},[['PUT','/rooms/closeds/654632876',{room:{name:'Designers'},user_ids:'EXISTING'}]]],
 ['add','room.membership.change',{type:'direct',members:[127326141,149087659,712064548]},[['POST','/rooms/directs/ROOM/add_members',{user_ids:[773523953]}]]],
 ['add_none','room.membership.change',{type:'direct',members:[127326141,149087659,712064548]},[['POST','/rooms/directs/ROOM/add_members',{user_ids:[149087659]}]]],
 ['last_leave','room.destroy',{type:'direct',name:'Weekend Plans',members:[127326141,149087659,712064548]},[['DELETE','/rooms/directs/ROOM/leave',{},127326141],['DELETE','/rooms/directs/ROOM/leave',{},149087659],['DELETE','/rooms/directs/ROOM/leave',{},712064548]]],
 ['leave_no_destroy','room.destroy',{type:'direct',members:[127326141,149087659,712064548]},[['DELETE','/rooms/directs/ROOM/leave',{}]]],
 ['leave_channel','room.membership.change',{designers:true},[['DELETE','/rooms/654632876/leave',{},773523953]]],
 ['leave_group','room.membership.change',{type:'direct',members:[127326141,149087659,712064548]},[['DELETE','/rooms/directs/ROOM/leave',{}]]],
 ['settings','account.settings.change',nil,[['PATCH','/account',{account:{name:'Renamed Account',settings:{restrict_room_creation_to_administrators:true}}}]]],
 ['styles','account.custom_styles.change',nil,[['PUT','/account/custom_styles',{account:{custom_styles:'body { color: red; }'}}]]],
 ['styles_same','account.custom_styles.change',{styles:'body { color: red; }'},[['PUT','/account/custom_styles',{account:{custom_styles:'body { color: red; }'}}]]]
]
def browser_for(id)
 user=User.find(id);browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
 request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
 request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax}
 browser.cookies['session_token']=request.cookie_jar[:session_token]
 browser.get '/users/me/profile';ActiveSupport::IsolatedExecutionState.clear
 token=Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
 if id==127326141
  browser.post '/sudo',params:{password:'secret123456'},headers:{'X-CSRF-Token'=>token};ActiveSupport::IsolatedExecutionState.clear
  raise 'sudo' unless browser.response.status==302
 end
 [browser,token]
end
cases=specs.map do |name,action,setup,steps|
 row=nil
 Rails.cache.clear # Fresh native app per declaration: isolate rate/cache state too.
 ActiveRecord::Base.transaction(requires_new:true) do
  room=nil
  if setup && setup[:type]
   room=(setup[:type]=='direct' ? Rooms::Direct : Rooms::Closed).create_for({name:setup[:name],creator:User.find(127326141)},users:User.where(id:setup[:members]).to_a)
  elsif setup && setup[:designers]
   room=Room.find(654632876)
  end
  if setup && setup[:replace]
   room.memberships.where(user_id:712064548).delete_all
   remove=room.users.where.not(id:127326141).first
   setup=setup.merge(removed:remove.id,existing:room.user_ids,changed:room.user_ids-[remove.id]+[712064548])
  elsif setup && setup[:designers]
   setup=setup.merge(existing:room.user_ids)
  end
  Account.first.update_columns(custom_styles:setup[:styles]) if setup && setup[:styles]
  AuditLog.delete_all
  browsers={};responses=[];requests=[]
  steps.each do |method,path,params,viewer|
   viewer||=127326141;browsers[viewer]||=browser_for(viewer);browser,token=browsers[viewer]
   path=path.gsub('ROOM',room.id.to_s) if room
   if params[:user_ids]=='REVISED' then params=params.merge(user_ids:setup[:changed]) elsif params[:user_ids]=='EXISTING' then params=params.merge(user_ids:setup[:existing]) end
   count=AuditLog.where(action:action).count
   browser.public_send(method.downcase,path,params:params,as: :json,headers:{'X-CSRF-Token'=>token,'Accept'=>'text/html'})
   ActiveSupport::IsolatedExecutionState.clear
   raise "transport #{name}: #{browser.response.status}" if browser.response.status>=500
   requests << {method:method,path:path,params:params,viewer:viewer}
   responses << {delta:AuditLog.where(action:action).count-count,entries:AuditLog.where(action:action).order(:id).map{|e|e.attributes.slice('actor_id','actor_label','target_id','target_type','target_label','details')}}
   responses.last[:status]=browser.response.status if name=='failed'
  end
  row={name:name,action:action,setup:setup,requests:requests,responses:responses}
  raise ActiveRecord::Rollback
 end
 row
end
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),cases:cases)
warn "Rails original room audit oracle: #{cases.size} declarations; #{cases.sum{|c|c[:responses].size}} routed audit observations"
