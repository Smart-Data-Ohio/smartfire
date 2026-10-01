GUARDS={'app/views/rooms/directs/new.html.erb'=>'58991330c09da9495f067f756a1284fd9c88892fdd13cac6f9165821219e5d94', 'app/views/rooms/directs/edit.html.erb'=>'8473f6ae48377bf21e5f01ae29ec76b11c06177bf3cb6a4d731db545d2a06f78', 'app/views/shared/_multi_select_bar.html.erb'=>'166856669367b007b7e5b4ff68bdc17f1fa3376c18bc19226676e2783a2e479a', 'app/controllers/rooms/directs_controller.rb'=>'46f1d745798c0a9003915c3a6c0b265d9f5079cad43812816b940a7decdcc460', 'app/models/rooms/direct.rb'=>'4947f936c7607738dd39b6b52f3b5f5adc071bde0d21cf801a05ac801bb6e6f1'}
require 'json'
require 'digest'
GUARDS.each { |path,hash|raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
class DirectFormsGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
controller=DirectFormsGoldenController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
h=controller.view_context
renderer=DirectFormsGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
user_view=->(user) {{id:user.id,name:user.name,title:user.title,avatar_url:h.fresh_user_avatar_path(user)}}
rows=[]
ActiveRecord::Base.transaction do
  david=User.find(127326141);kevin=User.find(712064548)
  group=Room.find(699448329)
  [[Room.find(186869642),david,'pair'],[Room.find(340026324),kevin,'pair_member'],[group,david,'group_admin'],[group,kevin,'group_member']].each do |room,viewer,name|
    Current.reset;Current.user=viewer
    users=room.users.many? ? room.users.without(viewer) : room.users
    candidates=User.active.ordered.where.not(id:room.user_ids).to_a
    edit={room_id:room.id,display_name:h.room_display_name(room),name:room.name,group_capable:room.group_capable?,administrator:viewer.administrator?,users:users.map(&user_view),candidates:candidates.map(&user_view),error_attributes:[]}
    rows << {name:name,kind:'edit',edit:edit,html:renderer.render(template:'rooms/directs/edit',layout:false,assigns:{room:room})}
  end
  [['invalid', ' x'*101],['blank',nil],['escaped','A <team> & "friends"']].each do |name,value|
    Current.reset;Current.user=david
    room=Room.find(group.id);room.name=value;room.valid?
    edit={room_id:room.id,display_name:h.room_display_name(room),name:room.name,group_capable:room.group_capable?,administrator:true,users:room.users.without(david).map(&user_view),candidates:User.active.ordered.where.not(id:room.user_ids).map(&user_view),error_attributes:room.errors.attribute_names}
    rows << {name:name,kind:'edit',edit:edit,html:renderer.render(template:'rooms/directs/edit',layout:false,assigns:{room:room})}
  end
  solo=Rooms::Direct.create_for({},users:[david]);Current.user=david
  rows << {name:'solo',kind:'edit',edit:{room_id:solo.id,display_name:h.room_display_name(solo),name:nil,group_capable:false,administrator:true,users:solo.users.map(&user_view),candidates:[],error_attributes:[]},html:renderer.render(template:'rooms/directs/edit',layout:false,assigns:{room:solo})}
  all=Rooms::Direct.create_for({},users:User.active.to_a)
  rows << {name:'all_members',kind:'edit',edit:{room_id:all.id,display_name:h.room_display_name(all),name:nil,group_capable:true,administrator:true,users:all.users.without(david).map(&user_view),candidates:[],error_attributes:[]},html:renderer.render(template:'rooms/directs/edit',layout:false,assigns:{room:all})}
  [['picker',[]],['starred_picker',[kevin.id]]].each do |name,stars|
    Current.reset;Current.user=david
    users=User.active.ordered.where.not(id:david.id).to_a.partition { |u|stars.include?(u.id) }.flatten
    picker=users.map { |u|{user:user_view.call(u),bot:u.bot?,agent:u.agent.present?,starred:stars.include?(u.id)} }
    rows << {name:name,kind:'new',users:picker,html:renderer.render(template:'rooms/directs/new',layout:false,assigns:{room:Rooms::Direct.new,users:users,starred_ids:stars})}
  end
  Current.reset;raise ActiveRecord::Rollback
end
puts JSON.pretty_generate({reference:'d7c7de92',forms:rows})
warn "Rails direct forms: #{rows.size} complete body goldens; reference d7c7de92"
