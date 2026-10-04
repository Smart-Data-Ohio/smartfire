HASHES={'app/controllers/rooms_controller.rb' => '53c7fd8b0619e478425d492f1be3abbc11affafa1b992dea09f8cd0b413c94eb', 'app/views/rooms/join.html.erb' => '6a5c3910c115b6037eea61da3f612c48773cc64544bdfbc5024ce503a5b43166'}
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
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,message,**_| frames << {stream:stream,html:message} if stream.end_with?(':rooms') }
cases={}
room=Room.find(201306877)
room.memberships.where(user_id:user.id).delete_all
client.get("/rooms/#{room.id}")
ActiveSupport::IsolatedExecutionState.clear
cases['preview']={status:client.response.status,last_room:client.cookies['last_room'],has_join:client.response.body.include?('Join channel')}
['join','repeat'].each do |key|
  frames.clear
  client.post("/rooms/#{room.id}/join",headers:{'X-CSRF-Token'=>token})
  ActiveSupport::IsolatedExecutionState.clear
  membership=Membership.find_by!(room_id:room.id,user_id:user.id)
  cases[key]={status:client.response.status,location:client.response.headers['Location'],involvement:membership.involvement,unread:membership.unread?,frames:frames.map(&:dup)}
end
room.memberships.where(user_id:user.id).delete_all
%w[Rooms::Closed Rooms::Direct Rooms::Voice Rooms::Stage Rooms::Board].each do |type|
  room.update_columns(type:type)
  frames.clear
  client.post("/rooms/#{room.id}/join",headers:{'X-CSRF-Token'=>token})
  ActiveSupport::IsolatedExecutionState.clear
  cases[type]={status:client.response.status,location:client.response.headers['Location'],frames:frames.map(&:dup)}
end
room.update_columns(type:'Rooms::Open',deleted_at:Time.current)
client.post("/rooms/#{room.id}/join",headers:{'X-CSRF-Token'=>token})
ActiveSupport::IsolatedExecutionState.clear
cases['deleted']={status:client.response.status,location:client.response.headers['Location']}
class RoomJoinGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Current.reset;Current.user=user
room=Room.find(654632876)
room.update_columns(name:'Join <&>')
renderer=RoomJoinGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
parts=JSON.parse(renderer.render(inline:'<% body=render template: "rooms/join" %><%= {body:body,nav:content_for(:nav)}.to_json.html_safe %>',layout:false,assigns:{room:room}))
puts JSON.pretty_generate({reference: ENV.fetch('PARITY_REFERENCE_SHA'),cases:cases,golden:{id:room.id,name:room.name,parts:parts}})
warn "Rails join oracle: #{cases.size} HTTP cases, 2 join-page regions; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
