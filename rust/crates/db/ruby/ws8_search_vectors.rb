require "json"
load "/tools/load_fixtures.rb"
ActiveJob::Base.queue_adapter = :test
parser_inputs = [nil, "", "cats AND dogs", "from:@jz has:file launch", "from:jason hello", "in:#Designers hi", "from:@jz, in:#Designers) hi", "from:@!!! has:bogus hello", "has:FILE has:file has:image", "before:2026-13-45 hi", "before:2024-02-29 after:2025-01-01 on:2026-03-10", "before:2026-03-10 before:2026-03-11", "is:THREAD hello", "FROM:David hi", "xfrom:David hi", "from:David\tfrom:Jason hi", "from:\u00a0 has:file", "\u00a0from:David hi", "café δog e\u0301 _42 你好", "\" OR 1=1 --", "in:#_ from:@%", "from:@\" OR \"1\"=\"1 injection", "has:pin, on:2026-03-10,", "from: from:@ in:# is:no", "hello\n  world\u00a0again"]
parser = parser_inputs.map do |raw|
  q = SearchQuery.parse(raw)
  { raw:raw, parsed:{raw:q.raw, text:q.text, from_names:q.from_names, in_rooms:q.in_rooms, has_values:q.has_values, before_date:q.before_date&.iso8601, after_date:q.after_date&.iso8601, on_date:q.on_date&.iso8601, thread_only:q.thread_only?, filters:q.filters?, blank_query:q.blank_query?, text_tokens:q.text_tokens, match_expression:q.match_expression, chips:q.chips.map(&:to_h)} }
end
user = User.find(ActiveRecord::FixtureSet.identify("david"))
room = Room.find(ActiveRecord::FixtureSet.identify("designers"))
other = Room.find(ActiveRecord::FixtureSet.identify("watercooler"))
thread = room.channel_threads.create!(creator:user,name:"WS8 search thread")
hidden = Rooms::Closed.create_for({name:"WS8 hidden",creator:User.find(ActiveRecord::FixtureSet.identify("kevin"))},users:[User.find(ActiveRecord::FixtureSet.identify("kevin"))])
deleted = Rooms::Closed.create_for({name:"WS8 deleted",creator:user},users:[user])
rows = [
 {client:"ws8-root", creator:"david", room:"designers", body:"ws8search ordinary", date:"2026-03-09 12:00:00"},
 {client:"ws8-link", creator:"jz", room:"designers", body:'ws8search <a href="https://example.test">link</a>', date:"2026-03-10 12:00:00"},
 {client:"ws8-drive", creator:"jason", room:"watercooler", body:"ws8search drive", drive:true, date:"2026-03-11 12:00:00"},
 {client:"ws8-image", creator:"david", room:"designers", body:"ws8search image", image:true, date:"2026-03-10 12:00:00"},
 {client:"ws8-pin", creator:"david", room:"designers", body:"ws8search pin", pin:true, date:"2026-03-10 12:00:00"},
 {client:"ws8-thread", creator:"david", room:"designers", body:"ws8search thread", thread:true, date:"2026-03-10 12:00:00"},
 {client:"ws8-note", creator:"david", room:"designers", body:"ws8search note", system_note:true, date:"2026-03-10 12:00:00"},
 {client:"ws8-stream", creator:"david", room:"designers", body:"ws8search stream", streaming:true, date:"2026-03-10 12:00:00"},
 {client:"ws8-hidden", creator:"kevin", room:"hidden", body:"ws8search hidden", date:"2026-03-10 12:00:00"},
 {client:"ws8-deleted", creator:"david", room:"deleted", body:"ws8search deleted", date:"2026-03-10 12:00:00"}
]
rows.each do |r|
  target = {"designers"=>room,"watercooler"=>other,"hidden"=>hidden,"deleted"=>deleted}.fetch(r[:room])
  m = target.messages.create!(creator:User.find(ActiveRecord::FixtureSet.identify(r[:creator])), body:r[:body], client_message_id:r[:client], thread:r[:thread] ? thread : nil, system_note:!!r[:system_note], streaming:!!r[:streaming])
  m.update_columns(created_at:Time.zone.parse(r[:date]))
  m.drive_attachments.create!(file_id:"1a2b3c4d5e6f7g8h9i0j") if r[:drive]
  if r[:image]
    blob = ActiveStorage::Blob.create!(key:"ws8-search-image",filename:"test.png",content_type:"image/png",byte_size:1,checksum:"ndTkYSaMgDT1yFZOFVxnpg==",service_name:"local")
    ActiveStorage::Attachment.create!(name:"attachment",record:m,blob:blob)
  end
  MessagePin.pin!(message:m,pinner:user) if r[:pin]
end
deleted.update_columns(deleted_at:Time.current)
queries = ["ws8search", "from:@david ws8search", "from:@david from:@jz ws8search", "in:#design ws8search", "in:#Design in:#Water ws8search", "has:link ws8search", "has:file ws8search", "has:image ws8search", "has:pin ws8search", "before:2026-03-10 ws8search", "after:2026-03-10 ws8search", "on:2026-03-10 ws8search", "after:2026-03-09 before:2026-03-11 ws8search", "is:thread ws8search", "from:@jz in:#Designers ws8search", "from:@% ws8search", "in:#_ ws8search", "from:@nosuch ws8search", "has:image has:pin ws8search", "in:#WS8 from:@david", "from:@david"]
results=queries.map { |raw| {raw:raw, clients:SearchQuery.parse(raw).apply_to_messages(user.reachable_messages).ordered.pluck(:client_message_id)} }
45.times { |i| room.messages.create!(creator:user,body:"ws8paging #{i}",client_message_id:"ws8-page-#{i}") }
scope=SearchQuery.parse("ws8paging").apply_to_messages(user.reachable_messages)
pages=[]; cursor=nil
loop do
  window=(cursor ? scope.before(cursor) : scope).reorder(created_at: :desc,id: :desc).limit(Message::PAGE_SIZE+1).to_a
  messages=window.first(Message::PAGE_SIZE).reverse
  pages << {clients:messages.map(&:client_message_id), has_more:window.size>Message::PAGE_SIZE}
  break unless window.size>Message::PAGE_SIZE
  cursor=messages.first
end
File.write(ARGV.fetch(0),JSON.pretty_generate({parser:parser,rows:rows,results:results,pages:pages})+"\n")
puts "WS8 search vectors: #{parser.size} grammar, #{results.size} SQLite filters, #{pages.size} cursor windows"
