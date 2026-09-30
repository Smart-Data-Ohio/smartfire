require 'json'
# Composed request header regions, over the actual parity seed. The only
# substituted request values are the shared renderer's CSRF/nonce placeholders.
class WS13PageViewsController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
ApplicationController.allow_forgery_protection=true
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
viewer=User.find(127326141)
users=[viewer,User.find(712064548),User.find(149087659)]
cases=[]
[Rooms::Open,Rooms::Closed,Rooms::Direct,Rooms::Voice,Rooms::Stage].each do |klass|
  room=klass.create!(id:9001,name:klass==Rooms::Direct ? nil : 'WS13 <&> room',creator:viewer)
  room.memberships.delete_all
  members=users.each_with_index.map {|u,i|room.memberships.create!(id:9011+i,user:u,stage_role:klass==Rooms::Stage ? %w[host listener speaker][i] : nil)}
  [false,true].each do |live|
    if live
      session=Session.create!(id:-7013,user:viewer,token:'ws13-page-view-session')
      HuddleGrant.create!(id:17,identity:'ws13-page-view-identity',room_name:Huddle.room_name(room.id),session:session,membership:members.first,user:viewer,room:room,last_seen_at:Time.current)
      Stream.create!(id:40,room:room,membership:members.first,user:viewer,quality:'1080p15') if room.stage?
    end
    [false,true].each do |configured|
      configured ? ENV['LIVEKIT_API_SECRET']='ws13-fixture-api-secret' : ENV.delete('LIVEKIT_API_SECRET')
      (room.stage? ? users : [viewer]).each do |actor|
        Current.reset;Current.user=actor
        room.reload
        options={http_host:'campfire.test',https:false,'rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_r){'NONCE'}}
        renderer=WS13PageViewsController.renderer.new(options)
        html=renderer.render(inline:'<% render "rooms/show/nav", room: @room %><%= content_for(:nav) %>',assigns:{room:room})
        stage=if room.stage?
          {room_id:room.id,viewer_id:room.memberships.find_by!(user:actor).id,members:room.memberships.order(:id).map {|m|{id:m.id,user_id:m.user_id,name:m.user.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(m.user),administrator:m.user.administrator?,role:m.stage_role,hand:nil,muted:m.server_muted?}},live:live ? {id:40,membership_id:members.first.id,name:viewer.name,identity:'ws13-page-view-identity'} : nil}
        end
        input={room:{id:room.id,kind:klass.name.demodulize.downcase,name:room.name,display_name:ApplicationController.helpers.room_display_name(room,for_user:actor)},configured:configured,pins_count:0,involvement:room.memberships.find_by!(user:actor).involvement,participants:configured && live ? [{id:viewer.id,name:viewer.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(viewer)}] : [],stage:stage}
        cases << {name:"#{klass.name.demodulize.downcase}_#{live ? 'live' : 'quiet'}_#{configured ? 'configured' : 'disabled'}_#{actor.id}",input:input,html:html}
      end
    end
    Stream.where(id:40).delete_all;HuddleGrant.where(id:17).delete_all;Session.where(id:-7013).delete_all
  end
  room.memberships.delete_all;room.delete
end
puts JSON.pretty_generate({reference_pin:'d7c7de92',cases:cases})
