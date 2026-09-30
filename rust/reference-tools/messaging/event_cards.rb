require 'json'
Current.user=User.find(127326141)
room=Room.find(699448326)
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
venue=Rooms::Voice.create!(name:'Venue <&>')
venue.memberships.create!(user:Current.user)
messages=[];events=[];cases=[]
[
 {label:'simple',time_zone:'UTC'},
 {label:'venue_meet',time_zone:'America/New_York',ends_at:Time.current+26.hours,venue:,meet_link:'https://meet.google.com/abc-defg-hij?x=1&y=2'},
 {label:'cancelled_series',time_zone:'Hawaii',cancelled_at:Time.current,meet_link:'javascript:alert(1)',series:true}
].each do |attributes|
 label=attributes.delete(:label);series=attributes.delete(:series);cancelled=attributes.delete(:cancelled_at)
 m=room.messages.create!(creator:Current.user,markdown_source:'event card reference',client_message_id:"event-card-#{label}")
 event=Event.create!({room:,organizer:Current.user,title:'Planning <&> "quoted"',starts_at:Time.current+25.hours}.merge(attributes))
 event.update_columns(series_id:event.id) if series
 EventReference.find_or_create_by!(message:m,event:)
 messages<<m;events<<event
 cases<<{label:,message_id:m.id,html:renderer.render(partial:'rooms/events/cards',locals:{message:Message.with_rendering_details.find(m.id)})}
end
ids=messages.map(&:id).join(',');eids=events.map(&:id).join(',')
selects={'rooms'=>"id=#{venue.id}",'events'=>"id IN (#{eids})",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'event_references'=>"message_id IN (#{ids})"}
rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:,cases:)+"\n")
puts "WS8bm2 event cards Rails oracle: #{cases.size} populated containers; #{rows.size} fixture tables"
