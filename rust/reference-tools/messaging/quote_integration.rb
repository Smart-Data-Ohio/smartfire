# Actual pinned Rails quote containers and rows; no hand-authored expected HTML.
require 'json'
user=User.find(127326141)
Current.user=user
rooms=[Room.find(699448326),Room.find(486777696),Room.find(186869642)]
messages=[]
sources=rooms.map.with_index do |room,i|
  source=room.messages.create!(creator:user,markdown_source:"quote integration #{i} & <words>",client_message_id:"integration-source-#{i}")
  messages<<source
  source
end
quotes=[
  [rooms[0],sources[0]], [rooms[0],sources[1]], [rooms[2],sources[2]]
].map.with_index do |(room,source),i|
  quote=room.messages.create!(creator:user,markdown_source:"see /rooms/#{source.room_id}/@#{source.id}",client_message_id:"integration-quote-#{i}")
  messages<<quote
  quote
end
empty=rooms[0].messages.create!(creator:user,markdown_source:'nothing quoted',client_message_id:'integration-empty')
messages<<empty
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
containers=(quotes+[empty]).map{|m|{id:m.id,html:renderer.render(partial:'messages/message_links/cards',locals:{message:Message.with_rendering_details.find(m.id)})}}
ids=messages.map(&:id).join(',')
selects={'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'message_references'=>"message_id IN (#{ids})"}
rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:,containers:,sources:sources.map(&:id),quotes:quotes.map(&:id),empty:empty.id)+"\n")
puts "WS8bm2 quote integration Rails oracle: #{containers.size} containers; #{messages.size} messages; #{rows.size} fixture tables"
