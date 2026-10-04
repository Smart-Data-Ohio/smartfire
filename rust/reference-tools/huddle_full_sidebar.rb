require 'json'
# Entire post-#163 sidebar, real partials and seed data; no HTML masks.
class WS13FullSidebarController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
renderer=WS13FullSidebarController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
admin=User.find(127326141); member=User.find(712064548)
Account.first.logo.detach
# Ensure every row variant is present, including a named group DM and self DM.
Rooms::Direct.create_for({id:9201,name:'Named <&> group',creator:admin},users:[admin,member,User.find(149087659)])
Rooms::Direct.create_for({id:9202,creator:admin},users:[admin])
cases=[];parts=[]
user_input=->(u) {{id:u.id,name:u.name,avatar_path:Rails.application.routes.url_helpers.fresh_user_avatar_path(u)}}
[false,true].each do |configured|
  configured ? ENV['LIVEKIT_API_SECRET']='ws13-fixture-api-secret' : ENV.delete('LIVEKIT_API_SECRET')
  [false,true].each do |restricted|
    account=Account.first;account.settings.restrict_room_creation_to_administrators=restricted;account.save!
    [admin,member].each do |actor|
      [false,true].each do |organized|
        actor.memberships.update_all(favorite_position:nil,room_category_id:nil)
        RoomCategory.delete_all
        if organized
          category=actor.room_categories.create!(id:9301,name:'Project <&> team',position:1,collapsed:false)
          actor.room_categories.create!(id:9302,name:'Empty collapsed',position:2,collapsed:true)
          favorite=actor.memberships.joins(:room).find_by!(rooms:{type:'Rooms::Stage'})
          favorite.update_columns(favorite_position:1)
          favorite=actor.memberships.joins(:room).find_by!(rooms:{id:9201}) if actor==admin
          favorite.update_columns(favorite_position:2) if actor==admin
          categorized=actor.memberships.joins(:room).find_by!(rooms:{type:'Rooms::Closed'})
          categorized.update_columns(room_category_id:category.id,involvement:'muted')
          actor.memberships.joins(:room).find_by!(rooms:{type:'Rooms::Voice'}).update_columns(room_category_id:category.id)
          actor.memberships.find_by!(room_id:9201).update_columns(room_category_id:category.id)
        end
        # Each startup configuration is a separate cold-render scenario. A cache
        # populated with Huddle disabled is not carried into an enabled process.
        Rails.cache.clear
        Current.reset;Current.user=actor.reload
        all=actor.memberships.visible.with_ordered_room.to_a
        favorites=all.select(&:favorited?).sort_by {|m|[m.favorite_position,m.id]}
        rest=all-favorites
        direct=rest.select {|m|m.room.direct?}.sort_by {|m|m.room.updated_at}.reverse
        voice=rest.select {|m|m.room.voice?}
        categorized=rest.select {|m|m.room_category_id.present?}
        other=rest-direct-voice-categorized
        categories=actor.room_categories.ordered.to_a
        exclude=Membership.where(room_id:actor.rooms.directs.pluck(:id)).pluck(:user_id).uniq.including(actor.id)
        placeholders=User.active.where.not(id:exclude).order(:created_at).limit([20-exclude.count,0].max).to_a
        preloaded=Membership.where(room_id:actor.memberships.joins(:room).where(room:{type:"Rooms::Direct"}).select(:room_id)).includes(:user).group_by(&:room_id).transform_values {|list|list.map(&:user)}
        make_row=->(m) do
          r=m.room;members=r.direct? ? preloaded.fetch(r.id,[]).reject {|u|u.id==actor.id} : []
          members=[actor] if r.direct? && members.empty?
          label=r.direct? ? (members.many? || r.name.present? ? r.direct_display_name(members:members,for_user:actor) : members.first.name.split(' ').first) : r.name
          live=r.stage? ? r.live_stream : nil
          {id:r.id,kind:r.class.name.demodulize.downcase,name:label,raw_name:r.name,epoch:r.updated_at.to_fs(:epoch),members:members.map(&user_input),call:{id:r.id,name:label,stage:r.stage?,icon:nil,participants:configured ? HuddleGrant.participants_for(r).map(&user_input) : [],live:!!live,live_name:live&.user&.name||'',unread:m.unread?,muted:m.involved_in_muted?,membership:true,favorited:m.favorited?,favorite_position:m.favorite_position,category_id:m.room_category_id,can_delete:actor.administrator? || (!r.direct? || !r.group_capable?) && r.creator_id==actor.id}}
        end
        input={account_name:Account.first.name,logo_path:nil,actor:user_input.call(actor),configured:configured,can_create:actor.administrator? || !restricted,favorites:favorites.map(&make_row),channels:other.reject {|m|m.room.stage? || m.room.board?}.map(&make_row),boards:other.select {|m|m.room.board?}.map(&make_row),voice:voice.map(&make_row),stage:other.select {|m|m.room.stage?}.map(&make_row),direct:direct.map(&make_row),placeholders:placeholders.map(&user_input),categories:categories.map {|c|{id:c.id,name:c.name,collapsed:c.collapsed?,rows:categorized.select {|m|m.room_category_id==c.id}.map {|m|make_row.call(m).merge(name:m.room.name.to_s,category_row:true)}}}}
        assigns={favorite_memberships:favorites,direct_memberships:direct,voice_memberships:voice,categorized_memberships:categorized,other_memberships:other,room_categories:categories,direct_placeholder_users:placeholders}
        html=renderer.render(template:'users/sidebars/show',layout:false,assigns:assigns)
        cases << {name:"#{configured}_#{restricted}_#{actor.id}_#{organized}",input:input,html:html}
        if !configured && restricted && actor==member && !organized
          empty=assigns.transform_values {[]}
          cases << {name:'empty',input:input.merge(favorites:[],channels:[],boards:[],voice:[],stage:[],direct:[],placeholders:[],categories:[]),html:renderer.render(template:'users/sidebars/show',layout:false,assigns:empty)}
        end
        if parts.empty? && !configured && !restricted && actor==admin && !organized
          all.each do |m|
            r=m.room
            next if r.voice? || r.stage?
            partial=r.direct? ? 'direct' : r.board? ? 'board' : 'shared'
            parts << {partial:partial,row:make_row.call(m),html:renderer.render(partial:"users/sidebars/rooms/#{partial}",locals:{room:r,membership:m,unread:m.unread?})}
          end
          parts << {partial:'menu',html:renderer.render(partial:'users/sidebars/room_menu')}
          parts << {partial:'placeholder',user:user_input.call(placeholders.first),html:renderer.render(partial:'users/sidebars/rooms/direct_placeholder',locals:{user:placeholders.first})}
        end
        if organized && !configured && !restricted && actor==admin
          categories.each do |c|
            parts << {partial:'category',category:input[:categories].find {|item|item[:id]==c.id},html:renderer.render(partial:'users/sidebars/room_categories',locals:{categories:[c],memberships:categorized.select {|m|m.room_category_id==c.id}})}
          end
        end
      end
    end
  end
end
puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],sidebar_revision:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases,parts:parts})
