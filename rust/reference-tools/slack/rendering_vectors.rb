# Actual Message saves, including the persisted Action Text body and mention resolution.
require 'json'
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join('db/schema.rb')
owner=User.create!(id:1,name:'Run owner')
jane=User.create!(id:2,name:'Jane Doe',email_address:'conv-jane@example.com')
room=Rooms::Closed.create!(id:1,name:'Render fixture',creator:owner)
room.memberships.grant_to([owner,jane])
users={'U001'=>'Jane Doe','U002'=>'Kevin'}
texts=["hi <@U001>!", "<!here> standup", "```const x = 1\nputs x\n```\n\nsee https://example.com/docs", "*bold* _it_ ~gone~\n\n• item\n\nsee <https://example.com|docs>"]
cases=texts.map do |text|
 source=Slack::MarkdownConverter.convert({'text'=>text},users:).markdown
 message=room.messages.create!(creator:owner,markdown_source:source)
 {text:,markdown:source,body:message.body.body.to_html,mentionees:message.mentionees.pluck(:id)}
end
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/rendering.json'),JSON.pretty_generate({users:,cases:})+"\n")
puts "Slack real-save rendering: #{cases.size} persisted body and mention goldens generated"
