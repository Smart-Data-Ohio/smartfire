# Complete open/closed form partials, including membership controls, from pinned Rails.
require 'json'
require 'digest'
{
 'app/views/rooms/opens/_form.html.erb'=>'03ae8882c4b8e4229c192c917570a356bbaa18c6730615db0c576abfcf348134',
 'app/views/rooms/closeds/_form.html.erb'=>'b3f00ae9f3659df0b361b3148fa75b9f8390f870827e73882a3960a516b3caca',
 'app/views/rooms/opens/_user.html.erb'=>'536d5c585240a2ae6ac63384d22e3870385d947aa3ad1be0a40de9894dba4220',
 'app/views/rooms/closeds/_user.html.erb'=>'1e0319164c2a08175eda3d71a6cde9c1285208be590e835f486e4cbbc044757b',
}.each { |path,hash|raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
class RoomFormsGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
controller=RoomFormsGoldenController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
h=controller.view_context
renderer=RoomFormsGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
users=User.active.ordered.to_a
user_view=->(user) {{id:user.id,name:user.name,title:user.title,avatar_url:h.fresh_user_avatar_path(user)}}
rows=[]
[['opens',Rooms::Open],['closeds',Rooms::Closed]].each do |namespace,klass|
  [['new',nil,127326141,nil],['invalid',nil,127326141,'missing_icon_ws8br'],['edit',201306877,127326141,'github'],['member',201306877,712064548,'smile'],['search',nil,127326141,nil]].each do |name,id,user_id,icon|
    Current.reset;Current.user=User.find(user_id)
    room=id ? Room.find(id).becomes!(klass) : klass.new(name:'New room',creator:Current.user)
    room.icon_name=icon
    room.valid?
    list=name=='search' ? users.cycle.take(21) : users
    selected_ids=id ? room.user_ids : []
    selected,unselected=list.partition { |user|selected_ids.include?(user.id) }
    locals={room:room,users:list,selected_users:selected,unselected_users:unselected,type_change_path:id ? "/rooms/#{namespace=='opens' ? 'closeds' : 'opens'}/#{id}/edit" : "/rooms/#{namespace=='opens' ? 'closeds' : 'opens'}/new"}
    html=renderer.render(partial:"rooms/#{namespace}/form",layout:false,locals:locals)
    rows << {name:"#{namespace}_#{name}",kind:namespace=='opens' ? 'open' : 'closed',type_change_path:locals[:type_change_path],form:{room:{id:id,name:room.name,icon_name:room.icon_name,errors:room.errors.full_messages,error_attributes:room.errors.attribute_names},can_administer:Current.user.can_administer?(room),current_user_id:user_id,users:list.map(&user_view),selected_users:selected.map(&user_view),unselected_users:unselected.map(&user_view)},html:html}
  end
end
Current.reset
puts JSON.pretty_generate({reference:'d7c7de92',forms:rows})
warn "Rails channel forms: #{rows.size} complete partial goldens (new, invalid, admin, member, search); reference d7c7de92"
