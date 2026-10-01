require "json"
require "active_record/fixtures"
ActiveRecord::Base.logger=nil
fixtures=Rails.root.join("test/fixtures")
ActiveRecord::FixtureSet.create_fixtures(fixtures,Dir[fixtures.join("**/*.yml")].map {|p|p.delete_prefix("#{fixtures}/").delete_suffix(".yml")},{"twitter_posts"=>Twitter::Post,"twitter_post_references"=>Twitter::PostReference})
ActiveJob::Base.queue_adapter=:test
class EventGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action&&method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
user=User.find(ActiveRecord::FixtureSet.identify(:david))
room=Room.find(ActiveRecord::FixtureSet.identify(:designers))
message=Message.new(id:601,client_message_id:"ws14e-golden-message",room:)
output={cards:[],attendances:[],meet_links:[]}
renderer=EventGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{})
[{}, {ends_at:nil}, {cancelled_at:Time.utc(2026,9,20)}, {series_id:401,recurrence_rule:"weekly"}, {venue:Rooms::Voice.new(id:501,name:"Calls & conversation")}, {meet_link:"https://meet.google.com/abc-defg-hij?x=1&y=2"}, {meet_link:"javascript:alert(1)"}, {title:"<script> & \"Planning\"",time_zone:"Eastern Time (US & Canada)"}].each_with_index do |attributes,index|
  ["UTC","Hawaii"].each do |zone|
    Time.use_zone(zone) do
      event=Event.new(id:401,room:,organizer:user,title:"Planning session",starts_at:Time.utc(2026,10,5,9),ends_at:Time.utc(2026,10,5,10),time_zone:"UTC",**attributes)
      Current.user=user
      html=renderer.render(partial:"rooms/events/card",locals:{event:,message:},layout:false)
      Current.reset
      output[:cards] << {name:"card #{index} #{zone}",viewer_zone:zone,event:{id:event.id,room_id:room.id,title:event.title,organizer_name:user.name,starts_at:event.starts_at.utc.iso8601,ends_at:event.ends_at&.utc&.iso8601,time_zone:event.time_zone,series:event.series?,cancelled:event.cancelled?,venue_name:event.venue&.name,meet_link:Object.new.extend(Rooms::EventsHelper).safe_meet_link(event)},html:}
    end
  end
end
head=room.events.create!(organizer:user,title:"Series",starts_at:Time.utc(2026,10,5,9),time_zone:"UTC",recurrence_rule:"weekly",recurrence_until:Date.new(2026,10,19))
[head,head.next_occurrence,head.series_events.last].each_with_index do |event,index|
  [nil,"Choose going, maybe, or declined.","<invalid> & alert"].each do |alert|
    Current.user=user
    html=renderer.render(template:"rooms/events/attendances/show",layout:false,assigns:{event:,room:,message_id:"601",frame_id:"response_for_message_601_event_#{event.id}",current_response:event.response_for(user),frame_alert:alert})
    output[:attendances]<<{event_id:event.id,room_id:room.id,message_id:"601",current_response:event.response_for(user),going:1,maybe:0,respondable:true,cancelled:false,apply_to_future: event.series_head?||event.next_occurrence.present?,alert:,html:}
    Current.reset
  end
end
[[0,0],[2,2],[3,0]].each do |going,maybe|
  EventAttendance.where(event:head).delete_all
  people=[:david,:jason,:jz,:kevin].map{|label|User.find(ActiveRecord::FixtureSet.identify(label))}
  (people.take(going).map{|u|[u,'going']}+people.drop(going).take(maybe).map{|u|[u,'maybe']}).each do |u,response|
    EventAttendance.insert_all!([{event_id:head.id,user_id:u.id,response:,created_at:Time.current,updated_at:Time.current}])
  end
  head=Event.find(head.id);Current.user=user
  response=head.response_for(user)
  output[:attendances] << {event_id:head.id,room_id:room.id,message_id:'601',current_response:response,going:,maybe:,respondable:true,cancelled:false,apply_to_future:true,alert:nil,html:renderer.render(template:'rooms/events/attendances/show',layout:false,assigns:{event:head,room:,message_id:'601',frame_id:"response_for_message_601_event_#{head.id}",current_response:response})}
  Current.reset
end
EventAttendance.where(event:head).delete_all
EventAttendance.insert_all!([{event_id:head.id,user_id:user.id,response:'going',created_at:Time.current,updated_at:Time.current}])
head=Event.find(head.id)
head.update_column(:cancelled_at,Time.utc(2026,9,20))
Current.user=user
output[:attendances]<<{event_id:head.id,room_id:room.id,message_id:nil,current_response:"going",going:1,maybe:0,respondable:false,cancelled:true,apply_to_future:true,alert:nil,html:renderer.render(template:"rooms/events/attendances/show",layout:false,assigns:{event:head,room:,message_id:nil,frame_id:"response_for_message__event_#{head.id}",current_response:"going"})}
Current.reset
%w[https://meet.google.com/a HTTPS://EXAMPLE.COM/a https://h:443/a http://h/a javascript:alert(1) https:/relative https://h/ü https://h/%zz https://h/?%zz https://h/?%z].each do |link|
  event=Event.new(meet_link:link)
  result=Object.new.extend(Rooms::EventsHelper).safe_meet_link(event)
  output[:meet_links]<<[link,result]
end
output[:collections]=[]
Time.use_zone("UTC") do
  event=Event.new(id:401,room:,organizer:user,title:"Planning session",starts_at:Time.utc(2026,10,5,9),ends_at:Time.utc(2026,10,5,10),time_zone:"UTC")
  second=Event.new(id:402,room:,organizer:user,title:"Second",starts_at:Time.utc(2026,10,6,9),time_zone:"UTC",cancelled_at:Time.utc(2026,9,20))
  [[[],"ws14e-golden-message"],[[second,event],"ws14e-golden-message"],[[event],%{bad"<&}]].each do |events,key|
    message.client_message_id=key
    message.association(:events).target=events
    message.association(:events).loaded!
    html=renderer.render(partial:"rooms/events/cards",locals:{message:},layout:false)
    facts=events.sort_by(&:starts_at).map {|e|{id:e.id,room_id:room.id,title:e.title,organizer_name:user.name,starts_at:e.starts_at.utc.iso8601,ends_at:e.ends_at&.utc&.iso8601,time_zone:e.time_zone,series:e.series?,cancelled:e.cancelled?,venue_name:nil,meet_link:nil}}
    output[:collections]<<{events:facts,message_key:key,html:}
  end
end
File.write("/rails/storage/db/event-fragments.json",JSON.pretty_generate(output)+"\n")
puts "Rails event HTML fragments: #{output[:cards].size} cards, #{output[:attendances].size} attendance frames"
