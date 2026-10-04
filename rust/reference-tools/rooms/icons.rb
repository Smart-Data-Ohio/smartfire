# Pinned room-icon HTTP writes and the real shared form layout. Tokens are lent to both
# renderers, as in views/core/goldens.rb; no template or icon implementation is replaced.
require 'json'
require 'digest'
{
  'app/models/room.rb'=>'9297f5aeffe5d78dca2b2d33f02173f56b918251fb398a2d17677fe3e85892ce',
  'app/models/icons.rb'=>'8b3169741b9569e8a56b0b634d1cd9907b08312b42844b2327bf5d751fd97df8',
  'app/controllers/rooms/opens_controller.rb'=>'932e1cc5ab97663253f5355cd2944f69779804a549d2ec14c62806c7f2717d0c',
  'app/controllers/rooms/closeds_controller.rb'=>'de11cf1268a4f84cb9d7d6b4dc972b4d6e27708404034ed4b6f133ec263973da',
  'app/views/rooms/layouts/_form.html.erb'=>'e55e3280dc4d1afe607a05ed8fe5d1709eef431f564c6a45160c0221b3ee8459',
  'app/views/shared/_icon_field.html.erb'=>'55ee26af4fe040d8f4548f3f113bba89ae5545be1fc265d3aed7c56a8fa94df7',
}.each { |path,hash| raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
WorkspaceIcon.insert_all!([{name:'ws8br_custom',title:'Custom icon',creator_id:127326141,created_at:Time.current,updated_at:Time.current}])
Icons.expire_custom_cache!
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
ActionCable.server.define_singleton_method(:broadcast) { |stream,message,**_|frames << {stream:stream,html:message} }
inputs=[':ws8br_custom:',"\u00a0:smile:\u00a0","\u2003github\u2003","\u0085slack\u0085",nil,''," \t:: GITHUB ::\r\n",':slack:',':smile:',"\v:SMILE:\v","\u00a0",':::','missing_icon_ws8br',true,false,123,[],{'name'=>'smile'}]
creations=[]
%w[opens closeds].each do |namespace|
  inputs.each do |input|
    frames.clear
    before={rooms:Room.count,memberships:Membership.count,audits:AuditLog.count}
    client.post("/rooms/#{namespace}",params:{room:{name:'Icon probe',icon_name:input},user_ids:[127326141]},headers:{'X-CSRF-Token'=>token,'Accept'=>'text/html'},as: :json)
    ActiveSupport::IsolatedExecutionState.clear
    creations << {namespace:namespace,input:input,status:client.response.status,icon_name:Room.count>before[:rooms] ? Room.last.icon_name : nil,delta:{rooms:Room.count-before[:rooms],memberships:Membership.count-before[:memberships],audits:AuditLog.count-before[:audits]},frames:frames.size}
  end
end
updates=[]
%w[opens closeds].each do |namespace|
  [':smile:','missing_icon_ws8br',nil,[],{'name'=>'smile'}].each do |input|
    Room.find(201306877).update_columns(name:'Baseline',type:'Rooms::Open',icon_name:'github')
    frames.clear
    room=Room.find(201306877)
    before={type:room.type,name:room.name,icon_name:room.icon_name,updated_at:room.updated_at.iso8601(6),members:room.user_ids.sort,audits:AuditLog.count}
    client.patch("/rooms/#{namespace}/#{room.id}",params:{room:{name:'Attempted',icon_name:input},user_ids:[127326141]},headers:{'X-CSRF-Token'=>token})
    ActiveSupport::IsolatedExecutionState.clear
    room=Room.find(room.id)
    updates << {namespace:namespace,input:input,status:client.response.status,before:before,after:{type:room.type,name:room.name,icon_name:room.icon_name,updated_at:room.updated_at.iso8601(6),members:room.user_ids.sort,audits:AuditLog.count},frames:frames.size,alert:client.response.body.include?('Icon name is not a known icon')}
  end
end
# Unchanged legacy unknown names are not revalidated; an omitted/filtered icon is untouched.
Room.find(201306877).update_columns(type:'Rooms::Open',icon_name:'legacy_unknown')
client.patch('/rooms/opens/201306877',params:{room:{name:'Legacy name',icon_name:':legacy_unknown:'}},headers:{'X-CSRF-Token'=>token})
ActiveSupport::IsolatedExecutionState.clear
legacy={status:client.response.status,icon_name:Room.find(201306877).icon_name}

class RoomIconGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
renderer=RoomIconGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
forms=[]
%w[Rooms::Open Rooms::Closed].each do |type|
  [nil,'github','smile','missing_icon_ws8br'].each do |icon|
    Current.reset;Current.user=user
    room=type.constantize.new(name:'Form <&>',creator:user,icon_name:icon)
    room.valid?
    html=renderer.render(inline:'<%= render layout: "rooms/layouts/form", locals: {room: @room} do %><p>Access fixture</p><% end %>',layout:false,assigns:{room:room})
    forms << {kind:room.open? ? 'open' : 'closed',can_administer:true,room:{id:nil,name:room.name,icon_name:room.icon_name,errors:room.errors.full_messages,error_attributes:room.errors.attribute_names},html:html}
  end
end
Current.reset
puts JSON.pretty_generate({reference: ENV.fetch('PARITY_REFERENCE_SHA'),creations:creations,updates:updates,legacy:legacy,forms:forms})
warn "Rails room icons: #{creations.size} creations, #{updates.size} updates, 1 legacy write, #{forms.size} exact form-layout goldens; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
