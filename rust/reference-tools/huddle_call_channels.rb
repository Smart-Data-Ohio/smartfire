require 'json'
# Real production Rails CRUD requests over the parity seed. No model/service tests.
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
ApplicationController.allow_forgery_protection=false
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
Huddle::CleanupJob.define_singleton_method(:perform_later) {|*_|}
users=[127326141,149087659,712064548].map {|id|User.find(id)}
users[1].update_columns(role: :member)
cases=[]
Room::DestroyJob.define_singleton_method(:perform_later) {|*_|}
deletions=[]
[Rooms::Voice,Rooms::Stage].each do |klass|
  namespace=klass==Rooms::Stage ? 'stages' : 'voices'
  scenarios={show:{action:'show'},new:{action:'new'},edit:{action:'edit'},create:{action:'create'},create_creator_omitted:{action:'create',ids:[1,2]},create_unknown_icon:{action:'create',icon:':notanicon:'},new_restricted:{action:'new',restricted:true,actor:2},create_restricted:{action:'create',restricted:true,actor:2},non_creator:{actor:2},creator_member:{actor:2,creator:2},update:{},update_icon:{icon:':FIRE:'},clear_icon:{icon:'',before_icon:'openai'},update_unknown_icon:{icon:':notanicon:'},unchanged_members:{ids:[0,1,2]},remove_member:{ids:[0,1]},remove_creator:{ids:[1,2]},empty:{ids:[]},outsider:{outsider:true},wrong_open:{wrong:'Rooms::Open'},wrong_closed:{wrong:'Rooms::Closed'},wrong_direct:{wrong:'Rooms::Direct'},wrong_other:{wrong:klass==Rooms::Stage ? 'Rooms::Voice' : 'Rooms::Stage'}}
  scenarios[:remove_host_with_successor]={ids:[1,2],second_host:true} if klass==Rooms::Stage
  scenarios.each do |name,opts|
    ActiveSupport::ExecutionContext.clear;Current.reset;Rails.cache.clear
    actor=users[opts.fetch(:actor,0)];creator=users[opts.fetch(:creator,0)]
    room=Room.create!(id:9001,type:opts.fetch(:wrong,klass.name),name:'Before',creator:creator,icon_name:opts[:before_icon])
    room.memberships.delete_all
    users.each_with_index {|u,i|room.memberships.create!(id:9011+i,user:u,stage_role:room.stage? ? (u==creator || opts[:second_host] && i==1 ? 'host' : 'listener') : nil)}
    room.memberships.find_by!(user:actor).delete if opts[:outsider]
    account=Account.first;old_restricted=account.settings.restrict_room_creation_to_administrators;account.settings.restrict_room_creation_to_administrators=!!opts[:restricted];account.save!
    request=ActionDispatch::Integration::Session.new(Rails.application);request.host! 'campfire.test'
    session=actor.sessions.create!(token: "ws13-call-#{namespace}-#{name}", two_factor_verified_at: Time.current, last_active_at: Time.current, user_agent: "WS13 Rails oracle", ip_address: "127.0.0.1")
    jar=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test")).cookie_jar
    jar.signed.permanent[:session_token]={value:session.token,httponly:true,same_site: :lax}
    request.cookies['session_token']=jar[:session_token]
    input={room:room.attributes,memberships:room.memberships.reload.map(&:attributes),users:users.map {|u|u.attributes.slice('id','role')},restricted:!!opts[:restricted]}
    action=opts.fetch(:action,'update')
    path="/rooms/#{namespace}";path+="/#{room.id}" unless %w[new create].include?(action);path+='/new' if action=='new';path+='/edit' if action=='edit'
    params={room:{name:'After'},user_ids:opts.fetch(:ids,[0,1,2]).map {|i|users[i].id}};params[:room][:icon_name]=opts[:icon] if opts.key?(:icon)
    verb={'new'=>'get','edit'=>'get','show'=>'get','create'=>'post','update'=>'put'}[action]
    request.public_send(verb,path,params:params,headers:{'HTTP_X_FORWARDED_PROTO'=>'https'})
    result=action=='create' && request.response.status==302 ? Room.find(request.response.headers['Location'].split('/').last) : room.reload
    errors=request.response.status==422 ? result.errors.full_messages : []
    expected_error=request.response.body[/Promote another host before removing [^<]+|Icon name is not a known icon/]
    cases << {name:"#{namespace}_#{name}",input:input,action:action,actor_id:actor.id,path:path,method:verb,params:params,request_body:request.request.raw_post,status:request.response.status,location:request.response.headers['Location'],error:expected_error,room:result.attributes.slice('type','name','icon_name','creator_id'),members:result.memberships.order(:user_id).map {|m|m.attributes.slice('user_id','stage_role')},notes:result.messages.where(system_note:true).count,audits:AuditLog.where(target_type:'Room',target_id:result.id).order(:id).map {|a|a.attributes.slice('action','details')}}
    result.memberships.delete_all;result.delete if result.id!=9001
    room.memberships.delete_all;room.delete
    AuditLog.where(target_type:'Room',target_id:[9001,result.id]).delete_all;Session.where(user_id:users.map(&:id)).delete_all;account.settings.restrict_room_creation_to_administrators=old_restricted;account.save!
  end
end
[Rooms::Voice,Rooms::Stage].each do |klass|
  room=klass.create_for({id:9001,name:'Delete me',creator:users[0]},users:users)
  Current.reset;Rails.cache.clear
  session=users[0].sessions.create!(token:"ws13-delete-#{klass.name}",two_factor_verified_at:Time.current,last_active_at:Time.current)
  jar=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test')).cookie_jar
  jar.signed.permanent[:session_token]={value:session.token,httponly:true,same_site: :lax}
  request=ActionDispatch::Integration::Session.new(Rails.application);request.host! 'campfire.test';request.cookies['session_token']=jar[:session_token]
  member=room.memberships.find_by!(user:users[0])
  grant=HuddleGrant.create!(id:17,user:users[0],session:session,membership:member,room:room,identity:'ws13-delete-identity',room_name:Huddle.room_name(room.id),last_seen_at:Time.current)
  stream=Stream.create!(id:40,room:room,user:users[0],membership:member,quality:'1080p15') if room.stage?
  request.delete('/rooms/9001',headers:{'HTTP_X_FORWARDED_PROTO'=>'https','Accept'=>'application/json'})
  room.reload;grant.reload;stream&.reload
  deletions << {stage:room.stage?,status:request.response.status,body:JSON.parse(request.response.body),deleted:!!room.deleted_at,claimed:!!room.destroy_enqueued_at,members:room.memberships.count,revoked:!!grant.revoked_at,stream_ended:stream ? !!stream.ended_at : nil,audits:AuditLog.where(target_type:'Room',target_id:room.id).order(:id).map {|a|a.attributes.slice('action','details')}}
  Stream.where(id:40).delete_all;HuddleGrant.where(id:17).delete_all;room.memberships.delete_all;room.delete;AuditLog.where(target_type:'Room',target_id:9001).delete_all
end
puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases,deletions:deletions})
