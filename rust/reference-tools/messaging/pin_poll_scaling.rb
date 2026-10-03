# Production pin-list and standalone poll partials, with uncached physical reads.
require 'json'
Current.user=User.find(127326141)
ActiveJob::Base.queue_adapter=:test
ActionCable.server.define_singleton_method(:broadcast) { |*_,**| }
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
room=Room.find(699448326)
def measured
 reads=[]
 observer=->(*args) { p=args.last; reads << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
 Icons.custom_icons
 icons_at=Icons.instance_variable_get(:@custom_cache_at)
 Icons.instance_variable_set(:@custom_cache_at,Float::INFINITY)
 html=ActiveRecord::Base.uncached { ActiveSupport::Notifications.subscribed(observer,'sql.active_record') { yield } }
 {html:html,reads:reads.size}
ensure
 Icons.instance_variable_set(:@custom_cache_at,icons_at)
end
groups=[]
[false,true].each do |populated|
[4,16].each do |size|
 room.message_pins.delete_all
 voters=size.times.map { |i| User.create!(name:"Voter #{populated}-#{size}-#{i} <&>",email_address:"voter-#{populated}-#{size}-#{i}@fixture.test",password:'fixture-password') }
 thread=ChannelThread.create!(room:room,creator:Current.user,name:"pin scaling #{size}")
 messages=size.times.map do |i|
  m=room.messages.create!(creator:User.find(i.odd? ? 149087659 : 127326141),thread:i.odd? ? thread : nil,markdown_source:"Pin #{size}-#{i}: #{populated ? '@[Bender Bot]' : 'ordinary'} :tada: #{'界'*205}",client_message_id:"pin-scale-#{populated}-#{size}-#{i}")
  MessagePin.create!(room:room,message:m,pinner:voters[i]);m
 end
 # Rendering resolves a fresh room, exactly like the production list callback.
 pins=measured { renderer.render(partial:'rooms/pins/list',locals:{room:Room.find(room.id)}) }
 poll_cases=[false,true].map do |anonymous|
  m=room.messages.create!(creator:Current.user,markdown_source:'Standalone poll',client_message_id:"poll-scale-#{populated}-#{size}-#{anonymous}")
  p=Poll.create_for_message!(message:m,labels:['A <&>','B'],anonymous:anonymous,multiple:true)
  voters.each_with_index { |v,i| PollVote.create!(poll:p,poll_option:p.poll_options[i%2],user:v) }
  html=measured { renderer.render(partial:'polls/poll',locals:{poll:Poll.find(p.id)}) }
  html.merge(poll_id:p.id,anonymous:anonymous,message_id:m.id)
 end
 message_ids=messages.map(&:id)+poll_cases.map { |p| p[:message_id] }
 poll_ids=poll_cases.map { |p| p[:poll_id] }
 selectors={'users'=>"id IN (#{voters.map(&:id).join(',')})",'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{message_ids.join(',')})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{message_ids.join(',')})",'message_pins'=>"message_id IN (#{message_ids.join(',')})",'polls'=>"id IN (#{poll_ids.join(',')})",'poll_options'=>"poll_id IN (#{poll_ids.join(',')})",'poll_votes'=>"poll_id IN (#{poll_ids.join(',')})"}
 rows=selectors.to_h { |table,where| [table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a.map { |r| table=='users' ? r.except('password_digest') : r }] }
 groups << {populated:populated,size:size,room_id:room.id,rows:rows,pins:pins,polls:poll_cases}
end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',groups:groups)+"\n")
puts "WS8bm2 pin/poll Rails scaling: #{groups.map { |g| "#{g[:populated]}:#{g[:size]} pins=#{g[:pins][:reads]} polls=#{g[:polls].map { |p| p[:reads] }.join('/')}" }.join(', ')}; 12 exact partials"
