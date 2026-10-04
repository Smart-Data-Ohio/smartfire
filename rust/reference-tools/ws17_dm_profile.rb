# Uncached DM wrapper and owned profile sections, rendered from the pinned ERB.
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
class Ws17DmProfileController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
env = {http_host: "campfire.test", https: false, "rack.session" => {},
 "action_dispatch.request.flash_hash" => ActionDispatch::Flash::FlashHash.new}
renderer=Ws17DmProfileController.renderer.new(env)
david=User.find_by!(email_address:"david@37signals.com")
jason=User.find_by!(email_address:"jason@37signals.com")
kevin=User.find_by!(email_address:"kevin@37signals.com")
dm=Rooms::Direct.find_by!(id:186869642)
group=Rooms::Direct.create_for({creator:kevin}, users:[david,jason,kevin])
channel=Rooms::Open.first!
rows=[]
travel_to(Time.utc(2026,3,2,16)) do
  states=[
   ["fixed_return",jason,dm,{david.id=>{ooo_until:Time.utc(2026,9,24,12),ooo_note:"Back soon"}}],
   ["escaped_note",jason,dm,{david.id=>{ooo_until:1.day.from_now,ooo_note:"<b>gone</b>"}}],
   ["viewer_jason",jason,dm,{david.id=>{ooo_until:1.day.from_now}}],
   ["viewer_david",david,dm,{david.id=>{ooo_until:1.day.from_now}}],
   ["group",kevin,group,{david.id=>{ooo_until:1.day.from_now},jason.id=>{ooo_until:2.days.from_now,ooo_note:"Slow to reply"}}],
   ["channel",jason,channel,{david.id=>{ooo_until:1.day.from_now}}],
   ["nobody_out",jason,dm,{}],
   ["invisible_manual",jason,dm,{david.id=>{presence_setting:"invisible",ooo_until:1.day.from_now,ooo_note:"Back soon"}}],
   ["invisible_calendar",jason,dm,{david.id=>{presence_setting:"invisible",ooo_calendar_enabled:true}}]
  ]
  states.each do |name,viewer,room,attrs|
    Calendar::MeetingCache.delete_all
    User.update_all(ooo_until:nil,ooo_note:nil,ooo_calendar_enabled:false,presence_setting:"auto",time_zone:"UTC")
    attrs.each { |id,changes| User.find(id).update!(changes) }
    if name=="invisible_calendar"
      Calendar::MeetingCache.create!(user:david, fetched_at:Time.current,ooo_intervals:[[5.minutes.ago.iso8601,2.days.from_now.iso8601]])
    end
    Current.user=viewer.reload
    members=room.direct? ? room.users.active.without_bots.where.not(id:viewer.id).includes(:meeting_cache).ordered.to_a : []
    data=members.map do |user|
      {id:user.id,name:user.name,visible:user.ooo_status_visible?,until_date:user.ooo_until_date,note:user.manual_ooo_active? ? user.ooo_note.presence : nil,stream_name:Turbo::StreamsChannel.signed_stream_name([user,:ooo_notice])}
    end
    rows << {name:,viewer_id:viewer.id,group:room==group,direct:room.direct?,attrs:attrs.transform_values { |changes| changes.transform_values { |value| value.respond_to?(:iso8601) ? value.iso8601 : value } },members:data,html:renderer.render(partial:"rooms/show/ooo_notices",locals:{room:,ooo_members:members})}
  end
  source=File.read(Rails.root.join("app/views/users/show.html.erb"))
  badge=source.lines.select { |line| line.include?("turbo_stream_from @user, :status") || line.include?("dom_id(@user, :status_badge)") }.join
  lines=source.lines; start=lines.index { |line| line.include?("if @dnd_allowed") }-1
  stop=(start+1...lines.size).find { |i| lines[i].include?("</div>") }
  allowance=lines[start..stop].join
  profiles=[]
  [["offline",{}],["custom",{custom_status_emoji:"🌱",custom_status_text:"<away & busy>"}],["dnd",{presence_setting:"dnd"}]].each do |name,attrs|
    User.update_all(ooo_until:nil,ooo_calendar_enabled:false,presence_setting:"auto",custom_status_emoji:nil,custom_status_text:nil,custom_status_expires_at:nil)
    david.reload.update!(attrs); Current.user=jason.reload
    profiles << {name:,user_id:david.id,presence:ApplicationController.helpers.display_presence_for(david),status_text:david.status_text_display,stream_name:Turbo::StreamsChannel.signed_stream_name([david,:status]),html:renderer.render(inline:badge,assigns:{user:david})}
  end
  allowances=[false,true].map { |allowed| {allowed:,html:renderer.render(inline:allowance,assigns:{user:david,dnd_allowed:allowed})} }
  assets=%w[notification-bell-everything.svg notification-bell-nothing.svg].to_h { |name| [name,ApplicationController.helpers.asset_path(name)] }
  broadcasts=[]
  ActiveJob::Base.queue_adapter=:test
  ActionCable.server.config.cable={"adapter"=>"test"}
  ["invisible_flip","ooo_end"].each do |name|
    travel_to(Time.utc(2026,3,2,16))
    User.update_all(meeting_status_enabled:false,ooo_calendar_enabled:false,ooo_until:nil,ooo_broadcast:nil)
    Calendar::MeetingCache.delete_all
    david.reload.update!(ooo_until:1.hour.from_now,presence_setting:name=="invisible_flip" ? "invisible" : "auto",custom_status_text:nil,custom_status_emoji:nil)
    david.reload.claim_ooo_broadcast!(true) if name=="ooo_end"
    travel_to(Time.utc(2026,3,2,18)) if name=="ooo_end"
    frames=[]
    sub=ActiveSupport::Notifications.subscribe("broadcast.action_cable") { |*args| p=args.last; frames << {stream:p[:broadcasting],html:p[:message]} }
    Calendar::OooDispatcher.dispatch_due!
    ActiveSupport::Notifications.unsubscribe(sub)
    broadcasts << {name:,now:Time.current.iso8601,frames:}
  end
  puts JSON.generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:"2026-03-02T16:00:00Z",rows:,profiles:,allowances:,assets:,broadcasts:)
end
Current.reset
