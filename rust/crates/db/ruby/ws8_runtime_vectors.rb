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
broadcasts=[{kind:"remove",stream:"gid://campfire/User/1:rooms",target:"list_rooms_direct_4",payload:ApplicationController.helpers.turbo_stream_action_tag(:remove,target:"list_rooms_direct_4")},{kind:"cable",stream:"user_1_unread_threads",payload:{threadId:4}}]
File.write(ARGV.fetch(0),JSON.pretty_generate({markdown:markdown,canonical:canonical,tasks:tasks,broadcasts:broadcasts,custom:{id:icon.id,name:icon.name,title:icon.title}})+"\n")
puts "WS8 runtime vectors: #{markdown.size} Markdown, #{canonical.size} canonicalization, #{tasks.size} periodic tasks, #{broadcasts.size} template-free broadcasts"
