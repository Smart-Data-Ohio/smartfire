require 'json'
# Owned forms/rows rendered over the pinned parity seed, with the same explicit
# CSRF placeholders as the shared views foundation. No HTML normalization.
class WS13CallViewsController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
users=User.active.ordered.to_a
viewer=User.find(127326141)
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
cases=[]
[Rooms::Voice,Rooms::Stage].each do |klass|
  type=klass==Rooms::Stage ? 'stages' : 'voices'
  [false,true].each do |persisted|
    room=persisted ? klass.create_for({id:9001,name:'WS13 <&> room',creator:viewer},users:users.take(2)) : klass.new(name:type=='stages' ? 'New stage channel' : 'New voice channel')
    Current.reset;Current.user=viewer
    selected=persisted ? users.select {|u|room.user_ids.include?(u.id)} : []
    unselected=users-selected
    options={http_host:'campfire.test',https:false,'rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_r){'NONCE'}}
    renderer=WS13CallViewsController.renderer.new(options)
    html=renderer.render(partial:"rooms/#{type}/form",locals:{room:room,selected_users:selected,unselected_users:unselected})
    input={id:room.id,name:room.name,stage:klass==Rooms::Stage,selected_users:selected.map {|u|{id:u.id,name:u.name,title:u.title,avatar_url:Rails.application.routes.url_helpers.fresh_user_avatar_path(u)}},unselected_users:unselected.map {|u|{id:u.id,name:u.name,title:u.title,avatar_url:Rails.application.routes.url_helpers.fresh_user_avatar_path(u)}}}
    cases << {name:"#{type}_#{persisted ? 'edit_form' : 'new_form'}",input:input,html:html}
    if !persisted
      page=renderer.render(template:"rooms/#{type}/new",layout:false,assigns:{room:room,users:users})
      cases << {name:"#{type}_new_page_content",input:input.merge(kind:'new_page'),html:page}
    end
    if persisted
      header=renderer.render(partial:'rooms/show/header_identity',locals:{room:room})
      cases << {name:"#{type}_header",input:input.merge(kind:'header',unread:false,membership:false,live:false,participants:[]),html:header}
      ['quiet','live'].each do |variant|
        if variant=='live'
          session=Session.create!(id:-7013,user:viewer,token:'ws13-call-view-session')
          member=room.memberships.find_by!(user:viewer)
          HuddleGrant.create!(id:17,identity:'ws13-call-view-identity',room_name:Huddle.room_name(room.id),session:session,membership:member,user:viewer,room:room,last_seen_at:Time.current)
          Stream.create!(id:40,room:room,membership:member,user:viewer,quality:'1080p15') if room.stage?
        end
        Current.reset;Current.user=viewer
        renderer=WS13CallViewsController.renderer.new(options)
        ['shared','member'].each do |scope|
          membership=scope=='member' ? room.memberships.find_by!(user:viewer) : nil
          locals={room:room,unread:scope=='member',membership:membership}
          row=renderer.render(partial:"users/sidebars/rooms/#{room.stage? ? 'stage' : 'voice'}",locals:locals)
          cases << {name:"#{type}_#{variant}_#{scope}",input:input.merge(kind:'row',unread:scope=='member',membership:!!membership,live:variant=='live',participants:variant=='live' ? [{id:viewer.id,name:viewer.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(viewer)}] : []),html:row}
        end
      end
      Stream.where(id:40).delete_all;HuddleGrant.where(id:17).delete_all;Session.where(id:-7013).delete_all
      room.memberships.delete_all;room.delete
    end
  end
end
layouts={}
Current.reset;Current.user=viewer
renderer=WS13CallViewsController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
%w[huddle huddle_invitation huddle_join_notice].each {|part|layouts[part]=renderer.render(partial:"layouts/#{part}")}
puts JSON.pretty_generate({reference_pin:'d7c7de92',cases:cases,layouts:layouts})
