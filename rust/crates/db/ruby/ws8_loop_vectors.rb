require_relative "ws8_vector_helpers"
before=dump
user=User.find(ActiveRecord::FixtureSet.identify("david"))
room=Room.find(ActiveRecord::FixtureSet.identify("designers"))
saved=[]; scheduled=[]; polls=[]
2.times do |i|
  msg=room.messages.create!(creator:user,markdown_source:"Loop #{i}")
  item=SavedItem.create!(user:user,message:msg,remind_at:1.hour.from_now)
  item.update_column(:remind_at,1.minute.ago); saved<<item.id
  draft=ScheduledMessage.create!(user:user,room:room,markdown_source:"Send #{i}",send_at:1.hour.from_now)
  draft.update_column(:send_at,1.minute.ago); scheduled<<draft.id
  question=room.messages.create!(creator:user,markdown_source:"Poll #{i}")
  poll=Poll.create_for_message!(message:question,labels:["one","two"],closes_at:1.hour.from_now)
  poll.update_column(:closes_at,1.minute.ago); polls<<poll.id
end
setup=delta(before)
triggers="CREATE TRIGGER ws8_saved_fault BEFORE UPDATE OF reminded_at ON saved_items WHEN NEW.id=#{saved.first} BEGIN SELECT RAISE(ABORT,'first saved'); END;\nCREATE TRIGGER ws8_scheduled_fault BEFORE UPDATE OF claimed_at ON scheduled_messages WHEN NEW.id=#{scheduled.first} BEGIN SELECT RAISE(ABORT,'first scheduled'); END;\nCREATE TRIGGER ws8_poll_fault BEFORE UPDATE OF closed_at ON polls WHEN NEW.id=#{polls.first} BEGIN SELECT RAISE(ABORT,'first poll'); END;"
triggers.split(";\n").each { |sql| C.execute(sql) }
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
SavedItem::ReminderDispatcher.dispatch_due!
ScheduledMessage::Dispatcher.dispatch_due!
Poll.close_due!
checks=[check("SELECT id,reminded_at IS NOT NULL FROM saved_items WHERE id IN (#{saved.join(',')}) ORDER BY id"),check("SELECT id,claimed_at IS NOT NULL,sent_at IS NOT NULL,dropped_at IS NOT NULL FROM scheduled_messages WHERE id IN (#{scheduled.join(',')}) ORDER BY id"),check("SELECT id,closed_at IS NOT NULL FROM polls WHERE id IN (#{polls.join(',')}) ORDER BY id"),check("SELECT source_id FROM activity_items WHERE source_type='SavedItem' AND source_id IN (#{saved.join(',')}) ORDER BY source_id")]
File.write(ARGV.fetch(0),JSON.pretty_generate({now:Time.current.to_fs(:db),setup_sql:setup+"\n"+triggers,checks:checks})+"\n")
puts "WS8 loop vectors: 3 first-row SQL failures, #{checks.size} continuation checks"
