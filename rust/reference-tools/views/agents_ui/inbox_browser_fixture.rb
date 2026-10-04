# ActivityInboxTest's setup, through the pinned Rails models. Each declaration
# gets its own seed copy; the second item is created only after handling the first.
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/agents_ui/labels.json')))
user = User.find(labels.fetch('users.david'))
ActivityItem.where(user:).delete_all
item = ActivityItem.create!(user:, source: Message.find(labels.fetch('messages.first')), event_type: 'mention')
output = {'system.activity_item' => item.id}
if ARGV.fetch(0) == 'inbox-filter'
  event = Room.find(labels.fetch('rooms.designers')).events.create!(
    organizer: User.find(labels.fetch('users.jason')), title: 'Filtered event',
    starts_at: 2.days.from_now, time_zone: 'UTC')
  output['system.event_item'] = ActivityItem.find_by!(user:, source: event).id
end
puts JSON.generate(output)
