require 'json'
# Real Rails view methods/partials, captured without HTML normalization.
class WS13RoomShellController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
actor=User.find(127326141)
Current.reset;Current.user=actor
renderer=WS13RoomShellController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
cases=[]
add=->(name,partial,input,html){cases << {name:name,partial:partial,input:input,html:html}}
[actor,User.find(712064548)].each do |u|
 Current.reset;Current.user=u
 add.call("pending_#{u.id}",'pending',{user:{id:u.id,name:u.name,title:u.title,avatar_url:Rails.application.routes.url_helpers.fresh_user_avatar_path(u)}},renderer.render(partial:'messages/template'))
end
Current.reset;Current.user=actor
add.call('preloads','head',{},renderer.render(inline:'<%= first_paint_controller_preloads %>'))
[Rooms::Open,Rooms::Closed,Rooms::Direct,Rooms::Voice,Rooms::Stage].each_with_index do |klass,i|
 room=klass.create_for({id:9201+i,name:'Room <&>',creator:actor},users:[actor,User.find(712064548)])
 input={room:{id:room.id,kind:klass.name.demodulize.downcase,name:room.name,display_name:room.name},updated_at:room.updated_at.iso8601(6)}
 [false,true].each do |scroll|
  add.call("area_#{i}_#{scroll}",'area',input.merge(scroll:scroll),renderer.render(inline:'<%= message_area_tag(@room, scroll_to_divider: (@scroll ? true : nil)) { "BODY".html_safe } %>',assigns:{room:room,scroll:scroll}))
 end
 add.call("list_#{i}",'list',input,renderer.render(inline:'<%= messages_tag(@room) { "BODY".html_safe } %>',assigns:{room:room}))
 room.memberships.delete_all;room.delete
end
[0,1,5,6,50].each do |count|
 add.call("unread_#{count}",'unread',{count:count},renderer.render(partial:'messages/unread_divider',locals:{unread_count:count}))
end
[nil,'/rooms/42?message_id=73'].each do |url|
 add.call("jump_#{url ? 'link' : 'button'}",'jump',{url:url},renderer.render(inline:'<%= button_to_jump_to_unread(url: @url) %>',assigns:{url:url}))
end
# Invitation must be the original room; no synthetic eligibility override.
room=Room.original
add.call('invitation','invitation',{join_code:Current.account.join_code},renderer.render(partial:'rooms/show/invitation',assigns:{room:room},locals:{room:room}))
# Per-viewer subscription set includes every active human other member, even while
# they are not OOO, so a later flip has a live target. Calendar note never leaks.
room=Rooms::Direct.create_for({id:9240,name:nil,creator:actor},users:[actor,User.find(712064548),User.find(149087659)])
member=User.find(712064548)
scenarios=[['quiet',nil,nil,'online','UTC',false,[]],['manual','2026-03-04T02:00:00Z','Trip <&>','online','Pacific Time (US & Canada)',false,[]],['invisible','2026-03-04T02:00:00Z','Trip <&>','invisible','UTC',false,[]],['calendar',nil,'No manual note','online','UTC',true,[['2026-03-02T00:00:00Z','2026-03-05T00:00:00Z']]],['overlap','2026-03-04T02:00:00Z','Manual <&>','online','UTC',true,[['2026-03-02T00:00:00Z','2026-03-05T00:00:00Z']]],['expired','2026-03-02T16:00:00Z','Expired note','online','UTC',false,[]]]
scenarios.each do |name,until_at,note,presence,zone,calendar,intervals|
 member.update_columns(ooo_until:until_at,ooo_note:note,presence_setting:presence,time_zone:zone,ooo_calendar_enabled:calendar)
 cache=Calendar::MeetingCache.find_or_initialize_by(user:member);cache.update!(ooo_intervals:intervals)
 [actor,member].each do |viewer|
  Current.reset;Current.user=viewer
  others=room.users.active.without_bots.where.not(id:viewer.id).includes(:meeting_cache).ordered.to_a
  input={room:{id:room.id,kind:'direct',name:nil,display_name:''},viewer_id:viewer.id,state:{member_id:member.id,ooo_until:until_at,ooo_note:note,presence:presence,zone:zone,calendar:calendar,intervals:intervals},notices:others.map {|u|{id:u.id,name:u.name,until_date:(u.ooo_until_date if u.ooo_status_visible?),note:(u.ooo_note.presence if u.ooo_status_visible? && u.manual_ooo_active?)}}}
  add.call("ooo_#{name}_#{viewer.id}",'ooo',input,renderer.render(partial:'rooms/show/ooo_notices',locals:{room:room,ooo_members:others}))
 end
end
puts JSON.pretty_generate({reference_pin:'d7c7de92',cases:cases})
