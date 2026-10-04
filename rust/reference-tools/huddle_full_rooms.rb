require 'json'
class WS13FullRoomsController < RoomsController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
viewer=User.find(127326141);other=User.find(712064548)
cases=[]
deferred=[]
[Rooms::Open,Rooms::Closed,Rooms::Direct,Rooms::Voice,Rooms::Stage].each_with_index do |klass,i|
 room=klass.create_for({id:9301+i,name:klass==Rooms::Direct ? nil : 'WS13 <&> room',creator:viewer},users:[viewer,other])
 [false,true].each do |configured|
  ENV.delete('LIVEKIT_API_SECRET')
  ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret') if configured
  [viewer,other].each do |actor|
   Current.reset;Current.user=actor
   notices=room.direct? ? room.users.active.without_bots.where.not(id:actor.id).includes(:meeting_cache).ordered.to_a : []
   members=room.memberships.order(:id).to_a
   stage=if room.stage?
    {room_id:room.id,viewer_id:room.memberships.find_by!(user:actor).id,members:members.map {|m|{id:m.id,user_id:m.user_id,name:m.user.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(m.user),administrator:m.user.administrator?,role:m.stage_role,hand:nil,muted:m.server_muted?}},live:nil}
   end
   r={id:room.id,kind:klass.name.demodulize.downcase,name:room.name,display_name:ApplicationController.helpers.room_display_name(room,for_user:actor)}
   input={room:r,updated_at:room.updated_at.iso8601(6),user:{id:actor.id,name:actor.name,title:actor.title,avatar_url:Rails.application.routes.url_helpers.fresh_user_avatar_path(actor)},messages:[],invitation:false,join_code:Current.account.join_code,messages_stream_name:Turbo::StreamsChannel.signed_stream_name([room,:messages]),navigation:{room:r,configured:configured,pins_count:0,involvement:room.memberships.find_by!(user:actor).involvement,participants:[],stage:stage},thread_panel_name:(room.direct? ? room.direct_display_name(for_user:nil) : room.name),shell:{notices:notices.map {|u|{id:u.id,name:u.name,until_date:nil,note:nil}}}}
   options={http_host:'campfire.test',https:false,'rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_r){'NONCE'}}
   renderer=WS13FullRoomsController.renderer.new(options)
   assigns={room:room,messages:[],ooo_notice_members:notices}
   html=renderer.render(template:'rooms/show',layout:'application',assigns:assigns)
   content=renderer.render(template:'rooms/show',layout:false,assigns:assigns)
   cases << {name:"#{r[:kind]}_#{configured}_#{actor.id}",input:input,configured:configured,html:html,content:content,providers:{vapid_public_key:Rails.configuration.x.vapid.public_key}}
  end
 end
 room.memberships.delete_all;room.delete
end
# Actual rooms from the tracked default seed, including populated timelines.
scenarios=[104393281,654632876,486777696,186869642,699448329,699448330,699448331].map {|id| [id,nil]}
scenarios += [[486777696,'onpage'],[486777696,'offpage']]
scenarios.each do |id,mode|
 room=Room.find(id)
 [false,true].each do |configured|
  ENV.delete('LIVEKIT_API_SECRET')
  ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret') if configured
  Rails.cache.clear
  Current.reset;Current.user=viewer
  messages=Message::MentionPreloader.preload_for(room.root_messages.with_rendering_details.last_page)
  unread_at=if mode
   first=mode=='onpage' ? messages[-7] : room.root_messages.ordered.first
   room.memberships.find_by!(user:viewer).update_columns(unread_at:first.created_at,last_read_message_id:nil)
   first.created_at.iso8601(6)
  end
  notices=room.direct? ? room.users.active.without_bots.where.not(id:viewer.id).includes(:meeting_cache).ordered.to_a : []
  controller=WS13FullRoomsController.new
  controller.set_request!(ActionDispatch::Request.new('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','SCRIPT_NAME'=>'','REQUEST_METHOD'=>'GET','PATH_INFO'=>"/rooms/#{id}"))
  controller.instance_variable_set(:@room,room)
  controller.instance_variable_set(:@messages,messages)
  controller.send(:set_unread_divider)
  unread_id=controller.instance_variable_get(:@unread_divider_message_id)
  unread_count=controller.instance_variable_get(:@unread_count) || 0
  scroll=!!controller.instance_variable_get(:@scroll_to_unread_divider)
  jump=controller.instance_variable_get(:@jump_to_unread_url)
  members=room.memberships.order(:id).to_a
  stage=if room.stage?
   {room_id:room.id,viewer_id:room.memberships.find_by!(user:viewer).id,members:members.map {|m|{id:m.id,user_id:m.user_id,name:m.user.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(m.user),administrator:m.user.administrator?,role:m.stage_role,hand:nil,muted:m.server_muted?}},live:nil}
  end
  r={id:room.id,kind:room.class.name.demodulize.downcase,name:room.name,display_name:ApplicationController.helpers.room_display_name(room,for_user:viewer)}
  input={room:r,updated_at:room.updated_at.iso8601(6),user:{id:viewer.id,name:viewer.name,title:viewer.title,avatar_url:Rails.application.routes.url_helpers.fresh_user_avatar_path(viewer)},messages:[],invitation:room==Room.original && !room.messages.paged?,join_code:Current.account.join_code,messages_stream_name:Turbo::StreamsChannel.signed_stream_name([room,:messages]),navigation:{room:r,pins_count:room.message_pins.count,involvement:room.memberships.find_by!(user:viewer).involvement,participants:[],stage:stage},thread_panel_name:(room.direct? ? room.direct_display_name(for_user:nil) : room.name),shell:{unread_message_id:unread_id,unread_count:unread_count,unread_index:unread_id && messages.index {|m|m.id==unread_id},scroll_to_divider:scroll,jump_url:jump,notices:notices.map {|u|{id:u.id,name:u.name,until_date:(u.ooo_until_date if u.ooo_status_visible?),note:(u.ooo_note.presence if u.ooo_status_visible? && u.manual_ooo_active?)}}}}
  options={http_host:'campfire.test',https:false,'rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_r){'NONCE'}}
  renderer=WS13FullRoomsController.renderer.new(options)
  assigns={room:room,messages:messages,ooo_notice_members:notices,unread_count:unread_count,unread_divider_message_id:unread_id,scroll_to_unread_divider:(scroll ? true : nil),jump_to_unread_url:jump}
  html=renderer.render(template:'rooms/show',layout:'application',assigns:assigns)
  content=renderer.render(template:'rooms/show',layout:false,assigns:assigns)
  fixture={name:"seed_#{id}_#{configured}#{mode ? "_#{mode}" : ''}",message_ids:messages.map(&:id),input:input,configured:configured,html:html,content:content,providers:{vapid_public_key:Rails.configuration.x.vapid.public_key}}
  fixture[:membership_unread_at]=unread_at if mode
  cases << fixture
 end
end
puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases,deferred:deferred})
