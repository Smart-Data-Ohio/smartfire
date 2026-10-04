require 'json'
class WS13FormPagesController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
Rails.application.routes.default_url_options[:host]='campfire.test'
ENV.update('LIVEKIT_URL'=>'wss://public.example.test','LIVEKIT_INTERNAL_URL'=>'http://internal.example.test:7880','LIVEKIT_API_KEY'=>'ws13-fixture-api-key','LIVEKIT_API_SECRET'=>'ws13-fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'ws13-fixture-gateway-secret')
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
users=User.active.ordered.to_a
viewer=User.find(127326141);member=User.find(712064548)
cases=[]
[Rooms::Voice,Rooms::Stage].each do |klass|
  type=klass==Rooms::Stage ? 'stages' : 'voices'
  [false,true].each do |persisted|
    room=persisted ? klass.create_for({id:9001,name:'WS13 <&> room',creator:viewer},users:[viewer,member]) : klass.new(name:klass==Rooms::Stage ? 'New stage channel' : 'New voice channel')
    variants=persisted ? %w[disabled create_address rotate_address repositories ordinary_member] : %w[administrator ordinary_member]
    variants.each do |variant|
      ENV.delete('INBOUND_EMAIL_DOMAIN')
      ENV['INBOUND_EMAIL_DOMAIN']='mail.example.test' if %w[create_address rotate_address].include?(variant)
      room.update_columns(inbound_email_token:variant=='rotate_address' ? 'a'*32 : nil) if persisted
      if variant=='repositories'
        Github::RepositorySubscription.insert_all!([{id:17,room_id:room.id,owner:'smart-data',repo:'app',events:%w[opened closed],created_at:Time.current,updated_at:Time.current},{id:18,room_id:room.id,owner:'basecamp',repo:'campfire',events:%w[merged checks_failed],created_at:Time.current,updated_at:Time.current}])
      end
      actor=variant=='ordinary_member' ? member : viewer
      Current.reset;Current.user=actor
      selected=persisted ? users.select {|u|room.user_ids.include?(u.id)} : []
      unselected=users-selected
      options={http_host:'campfire.test',https:false,'rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_r){'NONCE'}}
      renderer=WS13FormPagesController.renderer.new(options)
      assigns={room:room,users:users,selected_users:selected,unselected_users:unselected}
      template="rooms/#{type}/#{persisted ? 'edit' : 'new'}"
      html=renderer.render(template:template,layout:'application',assigns:assigns)
      content=renderer.render(template:template,layout:false,assigns:assigns)
      settings=persisted ? {room_id:room.id,can_administer:actor.can_administer?(room),administrator:actor.administrator?,repositories:room.github_repository_subscriptions.order(:owner,:repo).map {|r|r.attributes.slice('id','owner','repo','events')},inbound_domain:ENV['INBOUND_EMAIL_DOMAIN'],inbound_token:room.inbound_email_token} : nil
      input={id:room.id,name:room.name,stage:room.stage?,actor_id:actor.id,can_administer:actor.can_administer?(room),selected_users:selected.map {|u|{id:u.id,name:u.name,title:u.title,avatar_url:Rails.application.routes.url_helpers.fresh_user_avatar_path(u)}},unselected_users:unselected.map {|u|{id:u.id,name:u.name,title:u.title,avatar_url:Rails.application.routes.url_helpers.fresh_user_avatar_path(u)}},settings:settings,providers:{vapid_public_key:Rails.configuration.x.vapid.public_key,google_drive:!!actor.google_account&.drive?,recent_searches:actor.searches.ordered.limit(10).map {|r|{id:r.id,query:r.query}},notification_muted:actor.manual_dnd_active? || actor.presence_setting=='dnd',quiet_hours:actor.quiet_hours_enabled? && actor.quiet_hours_start_minute && actor.quiet_hours_end_minute ? [actor.quiet_hours_start_minute,actor.quiet_hours_end_minute] : nil,meeting_quiet:actor.meeting_dnd_enabled? && actor.meeting_status_enabled? ? actor.meeting_cache&.quiet_window_epochs.to_a : [],ooo_quiet:actor.ooo_notify_enabled? ? [] : ((actor.manual_ooo_active? ? [[0,actor.ooo_until.to_i]] : [])+(actor.ooo_calendar_enabled? ? actor.meeting_cache&.ooo_window_epochs.to_a : []))}}
      cases << {name:"#{type}_#{persisted ? 'edit' : 'new'}_#{variant}",input:input,html:html,content:content}
      Github::RepositorySubscription.where(room_id:room.id).delete_all if persisted
    end
    room.memberships.delete_all;room.delete if persisted
  end
end
puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases})
