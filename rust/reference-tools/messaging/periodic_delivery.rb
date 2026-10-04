# Actual registered periodic tasks: due precision, wide future dates and idempotence.
require 'json'
require 'active_support/testing/time_helpers'
require_relative 'oracle-database'
require_relative 'oracle-state'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter = :test
Random.define_singleton_method(:uuid) { 'fixture-periodic-message' }
Icons.custom_icons
Icons.instance_variable_set(:@custom_cache_at, Float::INFINITY)
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,html,**| frames << {stream:,html:} }
source=JSON.parse(File.read('/work/vectors/messaging/older_calendar_execution.json'))
reset=MessagingOracleDatabase.scenarios(ARGV.fetch(0))
groups=[]
source.fetch('groups').each do |group|
 %w[due wide_future wide_past normal_future].each do |kind|
  reset.call do
   travel_to Time.utc(2026,3,2,16)
   conn=ActiveRecord::Base.connection
   group.fetch('rows').each do |table,rows|
    rows.each { |row| conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |k| conn.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| conn.quote(v) }.join(',')})") }
   end
   user=User.find(127326141);user.update_columns(time_zone:'America/New_York')
   message=Message.find(group.fetch('old_ids').first);room=message.room
   at={'due'=>Time.current, 'wide_future'=>Time.iso8601('10000-03-05T09:00:00Z'), 'wide_past'=>Time.iso8601('-10000-03-05T09:00:00Z'), 'normal_future'=>1.hour.from_now}.fetch(kind)
   saved=SavedItem.create!(user:,message:)
   saved.update_columns(remind_at:at)
   scheduled=ScheduledMessage.create!(user:,room:,markdown_source:'Periodic scheduled <message> & proof',send_at:1.day.from_now)
   scheduled.update_columns(send_at:at)
   rows=group.fetch('rows').merge('saved_items'=>conn.select_all("SELECT * FROM saved_items WHERE id=#{saved.id}").to_a, 'scheduled_messages'=>conn.select_all("SELECT * FROM scheduled_messages WHERE id=#{scheduled.id}").to_a)
   tables=%w[messages action_text_rich_texts saved_items scheduled_messages activity_items]
   baseline=tables.to_h { |table| [table,conn.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }
   runner=Periodic::Runner.new
   # Keep the real registered task objects; isolate this consumer proof from unrelated owners.
   runner.instance_variable_get(:@tasks).select! { |task| ['saved item reminders','scheduled messages'].include?(task.name) }
   steps=[]
   [0,0,30].each do |elapsed|
    frames.clear;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    travel_to(Time.utc(2026,3,2,16)+elapsed)
    queries=[]
    observer=->(*args) { p=args.last;queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
    ran=nil
    Time.use_zone('UTC') { Current.reset;ActiveSupport::Notifications.subscribed(observer,'sql.active_record') { ran=runner.tick(now:Time.current) } }
    jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.map { |job| {class:job[:job].name, record_ids:job[:args].map { |argument| argument.fetch('_aj_globalid').split('/').last.to_i }} }
    steps << {elapsed:,ran:,state:MessagingOracleState.changes(baseline, tables.to_h { |table| [table, conn.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }),jobs:,frames:frames.select { |f| f[:stream]=="#{room.to_gid_param}:messages" }.dup,reads:queries.length}
   end
   groups << {size:group['size'],kind:,rows:,state_encoding:'rows_by_id_v1',baseline:,steps:}
  end
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],groups:)+"\n")
puts "WS8bm2 periodic Rails: #{groups.size} scenarios; #{groups.sum { |g| g[:steps].size }} real registered ticks; full rows, jobs and root frames"
