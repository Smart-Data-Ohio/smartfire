# Real pinned message callbacks and recorder. No stubs of policy, matcher or persistence.
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
ActiveJob::Base.queue_adapter=:test
ActionCable.server.config.cable={"adapter"=>"test"}
room=Room.find_by!(name:"Designers")
author=User.find_by!(email_address:"jz@37signals.com")
recipient=User.find_by!(email_address:"david@37signals.com")
rows=[]
travel_to(Time.utc(2026,3,2,16)) do
 scenarios=[
  {name:"keyword",body:"Shipping the DEPLOY now"},
  {name:"unicode_street",phrase:"straße",body:"STRASSE"},
  {name:"unicode_ligature",phrase:"office",body:"oﬃce"},
  {name:"unicode_sigma",phrase:"οσ",body:"ΟΣ"},
  {name:"unicode_dotted_i",phrase:"İ",body:"i̇"},
  {name:"unicode_combining_unequal",phrase:"é",body:"é"},
  {name:"unicode_turkish_unequal",phrase:"I",body:"ı"},
  {name:"boundary",body:"Redeploying the service"},
  {name:"self",self:true},
  {name:"inactive",inactive:true},
  {name:"bot",bot:true},
  {name:"dnd",attrs:{dnd_enabled:true}},
  {name:"quiet",attrs:{quiet_hours_enabled:true,quiet_hours_start_minute:900,quiet_hours_end_minute:1020,time_zone:"UTC"}},
  {name:"ooo",attrs:{ooo_until:"2026-03-03T16:00:00Z"}},
  {name:"mention",mention:true},
  {name:"reply",reply:true},
  {name:"reply_off",reply:true,reply_notify_author:false},
  {name:"mention_reply",mention:true,reply:true},
  {name:"streaming",streaming:true},
  {name:"system_note",system_note:true},
  *["invisible","nothing","muted","mentions","everything",nil,"missing"].map { |involvement| {name:"room_#{involvement || 'null'}",involvement:} },
  *["nothing","mentions","everything","missing"].map { |thread_involvement| {name:"thread_#{thread_involvement}",thread_involvement:} },
  {name:"thread_mention",thread_involvement:"everything",mention:true},
  {name:"thread_reply",thread_involvement:"everything",reply:true},
  {name:"thread_mention_reply",thread_involvement:"everything",reply:true,mention:true},
  {name:"thread_room_nothing",thread_involvement:"everything",involvement:"nothing"},
  {name:"thread_room_invisible",thread_involvement:"mentions",involvement:"invisible"},
  {name:"thread_room_missing",thread_involvement:"mentions",involvement:"missing"},
 ]
 scenarios.each_with_index do |row,i|
  recipient.update_columns(status:0,role:0,dnd_enabled:false,quiet_hours_enabled:false,ooo_until:nil,**row.fetch(:attrs,{}))
  recipient.keyword_alerts.delete_all
  member=room.memberships.find_or_initialize_by(user:recipient)
  member.involvement=row[:involvement]=="missing" ? "mentions" : row.fetch(:involvement,"mentions");member.save!
  recipient.update_columns(status:1) if row[:inactive]
  recipient.update_columns(role:2) if row[:bot]
  KeywordAlert.create!(user:recipient,phrase:row.fetch(:phrase,"deploy"))
  thread=nil
  if row[:thread_involvement]
   thread=ChannelThread.create!(room:,creator:author,name:"WS17 #{i}")
   ThreadMembership.join!(thread,author)
   ThreadMembership.join!(thread,recipient).update!(involvement:row[:thread_involvement]) unless row[:thread_involvement]=="missing"
  end
  room.memberships.where(user:recipient).delete_all if row[:involvement]=="missing"
  source=row[:reply] && room.messages.create!(creator:recipient,thread:,body:"Original",client_message_id:"ws17-source-#{i}")
  body=row.fetch(:body,"Deploy now")
  body+=" <action-text-attachment sgid=\"#{recipient.attachable_sgid}\" content-type=\"application/vnd.campfire.mention\"></action-text-attachment>" if row[:mention]
  attrs={creator:row[:self] ? recipient : author,body:,thread:,reply_to_message:source,reply_notify_author:row.fetch(:reply_notify_author,true),streaming:row[:streaming] || false,system_note:row[:system_note] || false,client_message_id:"ws17-activity-#{i}"}
  message=room.messages.new(attrs);message.importing=true if row[:importing];message.save!
  item=ActivityItem.find_by(user:recipient,source:message)
  # Candidates expose the scoped membership load used by the named ceiling assertions.
  recorder=ActivityItems::Recorder.new(message)
  rows << row.merge(body:,event_type:item&.event_type,candidate_ids:recorder.send(:room_memberships).keys)
 end
end
puts JSON.generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:)
