# Capture the owned shell regions and separately supplied owner fragments from our Rails.
require 'json'
require "digest"
{
  "app/views/rooms/show.html.erb" => "2218e078bc846e09238104d226f4f6674ef32aa4363002d438675fe48eb7e777",
  "app/views/rooms/show/_nav.html.erb" => "d6e42dbefc3a01164d2388d748b51c88f903607c8622c8a5d13a7185649be10d",
  "app/views/rooms/show/_member_panel.html.erb" => "67e6d30c798f4c24d6834cfbf264c06aaf2c0a284f9e9057e2e3e118a71d1c97",
  "app/views/rooms/show/_header_overflow.html.erb" => "ed06ff57f090334506b13e68442399a21ee85727dd163b4d19242bf422670769",
}.each { |path, hash| raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
class RoomShellGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
controller=RoomShellGoldenController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
h=controller.view_context
renderer=RoomShellGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_){'NONCE'})
rows=[]
ActiveRecord::Base.transaction do
  david=User.find(127326141); kevin=User.find(712064548)
  [[654632876,david,'empty_channel'],[186869642,david,'empty_pair'],[699448329,kevin,'empty_group']].each do |id,viewer,name|
    Current.reset;Current.user=viewer
    room=Room.find(id)
    # Only render empty collections, without modifying the pinned templates or message partials.
    membership=room.memberships.find_by!(user:viewer)
    assigns={room:room,messages:[],scroll_to_unread_divider:false,ooo_notice_members:[]}
    parts=JSON.parse(renderer.render(inline: '<% body = render template: "rooms/show" %><%= {body: body, head: content_for(:head), nav: content_for(:nav), member_panel: content_for(:member_panel), thread_panel: content_for(:thread_panel), footer: content_for(:footer)}.to_json.html_safe %>',layout:false,assigns:assigns))
    owned={pins_panel:renderer.render(partial:'rooms/pins/panel',locals:{room:room}),message_template:renderer.render(partial:'messages/template'),thread_panel:parts['thread_panel'],composer:parts['footer'],poll_builder:renderer.render(partial:'polls/builder',locals:{room:room})}
    rows << {name:name,room_id:room.id,param_key:room.model_name.param_key,room_name:room.name,display_name:h.room_display_name(room,for_user:viewer),kind:room.direct? ? 'direct' : room.open? ? 'open' : 'closed',involvement:membership.involvement || room.default_involvement,updated_at:room.updated_at.iso8601(6),user_id:viewer.id,user_name:viewer.name,avatar_path:h.fresh_user_avatar_path(viewer),signed_stream_name:Turbo::StreamsChannel.signed_stream_name([room,:messages]),parts:parts,owner_fragments:owned}
  end
  Current.reset
  raise ActiveRecord::Rollback
end
puts JSON.pretty_generate(rows)
warn "Rails room shell: #{rows.size} empty-room region goldens; reference d7c7de92"
