require 'json'
class WS13RowBroadcastsController < ApplicationController
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
actor=User.find(127326141);other=User.find(712064548);third=User.find(149087659)
cases=[]
headers=[]
renderer=WS13RowBroadcastsController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
[Rooms::Open,Rooms::Closed,Rooms::Direct].each_with_index do |klass,i|
 room=klass.create_for({id:9401+i,name:klass==Rooms::Direct ? nil : 'Row <&>',creator:actor},users:[actor,other,third])
 [false,true].each do |configured|
  ENV.delete('LIVEKIT_API_SECRET')
  ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret') if configured
  Current.reset;Current.user=actor
  if room.direct?
   room.memberships.order(:id).each do |member|
    [false,true].each do |unread|
     member.update_columns(unread_at:unread ? Time.current : nil,involvement:unread ? 'muted' : 'everything')
     members=room.users.where.not(id:member.user_id).to_a
     Current.reset
     html=renderer.render(partial:'users/sidebars/rooms/direct',locals:{membership:member,members:members,participants:[],huddleable:configured})
     cases << {name:"direct_#{member.user_id}_#{configured}_#{unread}",input:{room:{id:room.id,kind:'direct',name:room.name,display_name:room.direct_display_name(for_user:member.user)},actor_id:member.user_id,membership_id:member.id,unread:unread,involvement:member.involvement,member_ids:members.map(&:id),configured:configured},html:html}
    end
   end
  else
   # Controllers render a neutral shared row without a membership, even when
   # request Current.user is set. Delete/leave capabilities must remain false.
   Current.reset;Current.user=actor
   html=renderer.render(partial:'users/sidebars/rooms/shared',locals:{room:room})
   cases << {name:"#{klass.name.demodulize.downcase}_#{configured}",input:{room:{id:room.id,kind:klass.name.demodulize.downcase,name:room.name,display_name:room.name},configured:configured},html:html}
  end
 end
 if room.direct?
  [nil,'Group <&>'].each do |name|
   room.update_columns(name:name)
   [actor,other,third].each do |viewer|
    Current.reset
    html=renderer.render(partial:'rooms/show/header_identity',locals:{room:room,for_user:viewer})
    headers << {name:"header_#{viewer.id}_#{name.nil? ? 'unnamed' : 'named'}",room_id:room.id,actor_id:viewer.id,room_name:name,html:html}
   end
  end
 end
 room.memberships.delete_all;room.delete
end
puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases,headers:headers})
