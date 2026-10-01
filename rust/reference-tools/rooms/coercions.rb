HASHES={'app/controllers/rooms_controller.rb' => '53c7fd8b0619e478425d492f1be3abbc11affafa1b992dea09f8cd0b413c94eb', 'app/controllers/rooms/opens_controller.rb' => '932e1cc5ab97663253f5355cd2944f69779804a549d2ec14c62806c7f2717d0c', 'app/controllers/rooms/directs_controller.rb' => '46f1d745798c0a9003915c3a6c0b265d9f5079cad43812816b940a7decdcc460', 'app/controllers/rooms/closeds_controller.rb' => 'de11cf1268a4f84cb9d7d6b4dc972b4d6e27708404034ed4b6f133ec263973da'}
require 'json'
require 'digest'
HASHES.each { |path,hash|raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
user=User.find(127326141)
session=user.sessions.create!(two_factor_verified_at:Time.current)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:session.token,httponly:true,same_site: :lax}
client=ActionDispatch::Integration::Session.new(Rails.application)
client.host! 'campfire.test'
client.cookies['session_token']=request.cookie_jar[:session_token]
client.get('/users/me/profile')
ActiveSupport::IsolatedExecutionState.clear
token=Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')['content']


failures=[]
ActiveSupport::Notifications.subscribe("process_action.action_controller") { |*args| failures << args.last[:exception]&.first }
cases=[]
updates=[]
inputs=[false,true,42,1.25,nil,[1],{nested:'value'}," \u000b "]
%w[opens closeds].each do |namespace|
  inputs.each do |name|
    client.post("/rooms/#{namespace}",params:{room:{name:name},user_ids:[127326141]},headers:{'X-CSRF-Token'=>token},as: :json)
    ActiveSupport::IsolatedExecutionState.clear
    cases << {namespace:namespace,input:name,status:client.response.status,name:Room.last.name,user_ids:Room.last.user_ids.sort}
    Room.find(201306877).update_columns(name:'Cast baseline',type:'Rooms::Open')
    client.patch("/rooms/#{namespace}/201306877",params:{room:{name:name},user_ids:[127326141]},headers:{'X-CSRF-Token'=>token},as: :json)
    ActiveSupport::IsolatedExecutionState.clear
    updates << {namespace:namespace,input:name,status:client.response.status,name:Room.find(201306877).name}
  end
end
shows={}
['186869642','340026324','9999999999','nonsense'].each do |id|
  client.get("/rooms/directs/#{id}",headers:{'Accept'=>'application/json'})
  ActiveSupport::IsolatedExecutionState.clear
  shows[id]={status:client.response.status,json:(JSON.parse(client.response.body) rescue nil)}
end
ids=[]
[[127326141,149087659],127326141,127326141.75,nil,true,false,[[127326141]],[[127326141,149087659]],[[127326141],149087659],[[127326141],[149087659]],[[[127326141,149087659]]],{id:127326141}].each do |value|
  client.post('/rooms/closeds',params:{room:{name:'ID cast probe'},user_ids:value},headers:{'X-CSRF-Token'=>token},as: :json)
  ActiveSupport::IsolatedExecutionState.clear
  ids << {input:value,status:client.response.status,user_ids:Room.last.user_ids.sort}
end
formats={}
['application/json','application/xml','text/html','application/json,text/html','text/vnd.turbo-stream.html','*/*'].each do |accept|
  client.post('/rooms/closeds',params:{room:{name:'Format probe'},user_ids:[127326141]},headers:{'X-CSRF-Token'=>token,'Accept'=>accept},as: :json)
  ActiveSupport::IsolatedExecutionState.clear
  formats[accept]=client.response.status
end
stream_cases=[]
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,message,**_| frames << {stream:stream,html:message} if stream=='rooms' || stream.end_with?(':rooms') }
client.post('/rooms/closeds',params:{room:{name:'Format stream create'},user_ids:[127326141,149087659]},headers:{'X-CSRF-Token'=>token},as: :json)
ActiveSupport::IsolatedExecutionState.clear
room=Room.last
stream_cases << {name:room.name,status:client.response.status,json:JSON.parse(client.response.body),user_ids:room.user_ids.sort,frames:frames.dup}
frames.clear
client.patch("/rooms/closeds/#{room.id}",params:{room:{name:'Format stream update'},user_ids:[127326141,149087659]},headers:{'X-CSRF-Token'=>token},as: :json)
ActiveSupport::IsolatedExecutionState.clear
stream_cases << {name:room.reload.name,status:client.response.status,json:JSON.parse(client.response.body),user_ids:room.user_ids.sort,frames:frames.dup}
failure_class=failures.compact.last
begin
  client.post('/rooms/closeds',params:{room:{name:'Missing partial probe'},user_ids:[127326141]},headers:{'X-CSRF-Token'=>token},as: :json,env:{'action_dispatch.show_exceptions'=>:none})
rescue => error
  failure_class=error.class.name
end
puts JSON.pretty_generate({reference:'d7c7de92',names:cases,updates:updates,shows:shows,ids:ids,formats:formats,closed_json_failure_class:failure_class,stream_cases:stream_cases,integer_casts:{'true'=>ActiveRecord::Type.lookup(:integer).cast(true),'false'=>ActiveRecord::Type.lookup(:integer).cast(false)}})
warn "Rails room coercion oracle: #{cases.size} create casts, #{updates.size} update casts, #{shows.size} direct-show callbacks, #{formats.size} partial formats, #{ids.size} ID casts, #{stream_cases.size} stream failure cases; reference d7c7de92"
