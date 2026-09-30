require "json"
load "/tools/load_fixtures.rb"
ActiveJob::Base.queue_adapter=:test
user=User.find(ActiveRecord::FixtureSet.identify("david"));room=Room.find(ActiveRecord::FixtureSet.identify("designers"))
icon=WorkspaceIcon.new(name:"ws8custom",title:"WS8 custom",creator:user)
icon.image.attach(io:File.open(Rails.root.join("test/fixtures/files/workspace_icons/clean.svg")),filename:"clean.svg",content_type:"image/svg+xml")
icon.save!
sources=["# Title\n\n**bold** _emphasis_ ~~gone~~", "Hello @[David] and @[Jason] and @[Missing]", "`@[David] :openai:` and :gpt: :ws8custom: :smile:", "| one | two |\n| --- | --- |\n| alpha | beta |", "- [x] Done\n- [ ] Later", "[room](/rooms/1/@2) and https://example.com", "<script>alert(1)</script> **safe**", "@[Kevin] escaped \\@[David]"]
markdown=sources.map{|source|body=Message::Markdown.render(source,room:room);{source:source,body:body,plain:Message::Markdown.plain_text(ActionText::Content.new(body))}}
html=["<p>Hello</p>",'<action-text-attachment content-type="application/vnd.campfire.mention" sgid="bad" bad="drop"></action-text-attachment>', '<figure data-trix-attachment=\'{"content":"<b>x</b>","contentType":"text/html"}\' data-trix-attributes=\'{"caption":"caption"}\'></figure>',"<p>broken<div>markup</p></div>"]
canonical=html.map{|body|{input:body,output:ActionText::Content.new(body).to_html}}
runner=Periodic::Runner.new(reminders_interval:17,retention_interval:123)
tasks=runner.instance_variable_get(:@tasks).select{|t|["saved item reminders","scheduled messages","poll closing","stuck rooms","retention prune"].include?(t.name)}.map{|t|{name:t.name,seconds:t.interval.to_i}}
# Capture the callback's actual payload. Passing a handwritten payload through
# both runtimes only checks transport and cannot prove the domain contract.
thread = room.channel_threads.create!(creator: user, name: "WS8 runtime thread")
recipient = User.find(ActiveRecord::FixtureSet.identify("jason"))
thread.memberships.create!(user: recipient)
reply_source = "WS8 runtime reply"
captured = []
server = ActionCable.server
original_broadcast = server.method(:broadcast)
server.define_singleton_method(:broadcast) { |stream, payload, **| captured << {kind: "cable", stream: stream, payload: payload} }
begin
  room.messages.create!(thread: thread, creator: user, markdown_source: reply_source)
ensure
  server.define_singleton_method(:broadcast, original_broadcast)
end
unread = captured.select { |event| event[:stream] == UnreadThreadsChannel.stream_name_for(recipient.id) }
raise "expected one real unread-thread callback" unless unread.one?
broadcasts=[{kind:"remove",stream:"gid://campfire/User/1:rooms",target:"list_rooms_direct_4",payload:ApplicationController.helpers.turbo_stream_action_tag(:remove,target:"list_rooms_direct_4")},*unread]
thread_broadcast = {room_id: room.id, creator_id: user.id, recipient_id: recipient.id, name: thread.name, source: reply_source}
flow_source = "**Scheduled** @[David] and :gpt:"
flow_edit = "## Edited\n\n- [x] ready @[Jason]"
scheduled = ScheduledMessage.create!(user:user,room:room,markdown_source:flow_source,send_at:1.hour.from_now)
ScheduledMessage::Dispatcher.dispatch_now!(scheduled)
posted = scheduled.reload.sent_message
flows = {scheduled:{source:flow_source,body:posted.body.body.to_html,plain:posted.plain_text_body}}
posted.update!(markdown_source:flow_edit)
flows[:edited] = {source:flow_edit,body:posted.body.body.to_html,plain:posted.plain_text_body}
destination = Room.find(ActiveRecord::FixtureSet.identify("watercooler"))
forward = Messages::Forwarder.call(source:posted,destinations:[{room_id:destination.id}],note:"Note @[David]",creator:user).first.message
flows[:forwarded] = {body:forward.body.body.to_html,plain:forward.plain_text_body,markdown_source:forward.markdown_source,forwarded_markdown:forward.forwarded_markdown?,mentionees:forward.mentionees.map(&:id)}
attachment = room.messages.create!(creator:user,body:"Attachment")
attachment.attachment.attach(io:StringIO.new("WS8 copy\n"),filename:"ws8.txt",content_type:"text/plain",identify:false,metadata:{"ws8"=>"metadata"})
copy = Messages::Forwarder.call(source:attachment,destinations:[{room_id:destination.id}],creator:user).first.message.attachment.blob
attachment_copy = copy.attributes.slice("filename","content_type","byte_size","checksum","metadata","service_name")
File.write(ARGV.fetch(0),JSON.pretty_generate({markdown:markdown,canonical:canonical,tasks:tasks,broadcasts:broadcasts,thread_broadcast:thread_broadcast,custom:{id:icon.id,name:icon.name,title:icon.title},flows:flows,attachment_copy:attachment_copy})+"\n")
puts "WS8 runtime vectors: #{markdown.size} Markdown, #{canonical.size} canonicalization, #{tasks.size} periodic tasks, #{broadcasts.size} template-free broadcasts, #{flows.size} write flows, 1 attachment copy"
