# The interleaved ActivityItem.create! in activity_inbox_test.rb:29.
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/agents_ui/labels.json')))
item = ActivityItem.create!(user: User.find(labels.fetch('users.david')),
  source: Message.find(labels.fetch('messages.second')), event_type: 'reply')
puts JSON.generate(id: item.id)
