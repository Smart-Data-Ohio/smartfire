HASHES={'app/controllers/rooms_controller.rb' => '53c7fd8b0619e478425d492f1be3abbc11affafa1b992dea09f8cd0b413c94eb', 'app/controllers/rooms/opens_controller.rb' => '932e1cc5ab97663253f5355cd2944f69779804a549d2ec14c62806c7f2717d0c', 'app/controllers/rooms/closeds_controller.rb' => 'de11cf1268a4f84cb9d7d6b4dc972b4d6e27708404034ed4b6f133ec263973da'}
require 'json'
require 'digest'
Rails.logger = ActiveSupport::Logger.new($stderr)
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

frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,message,**_| frames << {stream:stream,html:message} if stream=='rooms' || stream.end_with?(':rooms') }
cases={}
def snapshot(client,room,frames)
  ActiveSupport::IsolatedExecutionState.clear
  room.reload
  {status:client.response.status,location:client.response.headers['Location'],room:{id:room.id,name:room.name,type:room.type,user_ids:room.user_ids.sort},audits:AuditLog.where(target_type:'Room',target_id:room.id).order(:id).map { |log|log.attributes.slice('action','actor_id','actor_label','target_type','target_id','target_label','details','ip_address') },frames:frames.map(&:dup)}
end
[['open','opens'],['closed','closeds']].each do |key,namespace|
  frames.clear
  client.post("/rooms/#{namespace}",params:{room:{name:"Audited #{key}"},user_ids:[127326141,149087659]},headers:{'X-CSRF-Token'=>token})
  cases[key]=snapshot(client,Room.last,frames)
end
open_room=Room.find(cases['open'][:room][:id])
frames.clear
client.patch("/rooms/opens/#{open_room.id}",params:{room:{name:'Renamed <&>'}},headers:{'X-CSRF-Token'=>token})
cases['open_update']=snapshot(client,open_room,frames)
room=Room.find(486777696)
room.memberships.where(user_id:712064548).delete_all
['revise','no_change'].each do |key|
  frames.clear
  client.patch("/rooms/closeds/#{room.id}",params:{room:{name:'Revised room'},user_ids:[127326141,149087659,712064548]},headers:{'X-CSRF-Token'=>token})
  cases[key]=snapshot(client,room,frames)
end
# A post-commit audit outage must not rewind the completed domain write.
original=AuditLog.method(:record!)
AuditLog.define_singleton_method(:record!) do |**args|
  if args[:action]=='room.create' || args[:action]=='room.membership.change'
    raise 'injected controller audit failure'
  end
  original.call(**args)
end
[['failed_open','opens'],['failed_closed','closeds']].each do |key,namespace|
  frames.clear
  client.post("/rooms/#{namespace}",params:{room:{name:"Audit failed #{key}"},user_ids:[127326141,149087659]},headers:{'X-CSRF-Token'=>token})
  cases[key]=snapshot(client,Room.last,frames)
end
frames.clear
client.patch("/rooms/closeds/#{room.id}",params:{room:{name:'Audit failed revision'},user_ids:[127326141,712064548]},headers:{'X-CSRF-Token'=>token})
cases['failed_revision']=snapshot(client,room,frames)
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases})
warn "Rails room audit oracle: #{cases.size} committed HTTP transitions; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
