require 'json'
# Render the real sidebar, then select its complete contiguous Voice/Stage
# section region by byte offsets. No HTML serialization, normalization or masks.
class WS13SidebarViewsController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
cases=[]
admin=User.find(127326141)
[false,true].each do |live|
  if live
    stage=Rooms::Stage.first
    member=stage.memberships.find_by!(user:admin)
    session=Session.create!(id:-7013,user:admin,token:'ws13-sidebar-session')
    HuddleGrant.create!(id:17,identity:'ws13-sidebar-identity',room_name:Huddle.room_name(stage.id),session:session,membership:member,user:admin,room:stage,last_seen_at:Time.current)
    Stream.create!(id:40,room:stage,membership:member,user:admin,quality:'1080p15')
  end
  [false,true].each do |restricted|
    account=Account.first;account.settings.restrict_room_creation_to_administrators=restricted;account.save!
    [admin,User.find(712064548)].each do |actor|
      Current.reset;Current.user=actor
      all=actor.memberships.visible.with_ordered_room.to_a
      favorites=all.select(&:favorited?).sort_by {|m|[m.favorite_position,m.id]}
      rest=all-favorites
      direct=rest.select {|m|m.room.direct?}.sort_by {|m|m.room.updated_at}.reverse
      voice=rest.select {|m|m.room.voice?}
      categorized=rest.select {|m|m.room_category_id.present?}
      other=rest-direct-voice-categorized
      assigns={favorite_memberships:favorites,direct_memberships:direct,voice_memberships:voice,categorized_memberships:categorized,other_memberships:other,room_categories:actor.room_categories.ordered.to_a,direct_placeholder_users:[]}
      renderer=WS13SidebarViewsController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
      html=renderer.render(template:'users/sidebars/show',layout:false,assigns:assigns)
      start=html.b.index('      <section class="sidebar-section sidebar-section--voice rooms"')
      finish=html.b.index('      <turbo-frame id="direct_rooms_control"',start)
      raise 'missing sidebar section boundaries' unless start && finish
      participants=(voice+other.select {|m|m.room.stage?}).to_h {|m|[m.room_id,HuddleGrant.participants_for(m.room)]}
      rows=(voice+other.select {|m|m.room.stage?}).map do |membership|
        room=membership.room;stream=room.stage? ? room.live_stream : nil
        {id:room.id,name:room.name,stage:room.stage?,icon:nil,participants:(participants[room.id]||[]).map {|u|{id:u.id,name:u.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(u)}},live:!!stream,live_name:stream&.user&.name||'',unread:membership.unread?,muted:membership.involved_in_muted?,membership:true,favorited:membership.favorited?,favorite_position:membership.favorite_position,category_id:membership.room_category_id,can_delete:actor.can_administer?(room)}
      end
      cases << {name:"#{live ? 'live' : 'quiet'}_#{restricted ? 'restricted' : 'open'}_#{actor.id}",input:{actor_id:actor.id,can_create:actor.administrator? || !restricted,rows:rows},html:html.byteslice(start,finish-start)}
    end
  end
  Stream.where(id:40).delete_all;HuddleGrant.where(id:17).delete_all;Session.where(id:-7013).delete_all
end
puts JSON.pretty_generate({reference_pin:'d7c7de92',cases:cases})
