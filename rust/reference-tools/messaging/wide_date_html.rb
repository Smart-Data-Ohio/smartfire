# Real saved/scheduled partials and HTML scheduling notice, with wide persisted years.
require 'json'
require 'active_support/testing/time_helpers'
require_relative 'oracle-database'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActiveJob::Base.queue_adapter = :test
Icons.custom_icons
Icons.instance_variable_set(:@custom_cache_at, Float::INFINITY)
source = JSON.parse(File.read('/work/vectors/messaging/older_calendar_execution.json'))
reset = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
groups = []
source.fetch('groups').each do |group|
 reset.call do
  conn = ActiveRecord::Base.connection
  group.fetch('rows').each do |table, rows|
   rows.each { |row| conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |k| conn.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| conn.quote(v) }.join(',')})") }
  end
  user = User.find(127326141)
  message = Message.find(group.fetch('old_ids').first)
  saved = SavedItem.create!(user:, message:)
  scheduled = ScheduledMessage.create!(user:, room: message.room, markdown_source: 'Wide scheduled <draft> & history', send_at: 1.day.from_now)
  cases = []
  %w[UTC America/New_York Australia/Lord_Howe Pacific/Apia].each do |zone|
   user.update_columns(time_zone: zone)
   ['9999-12-31T23:59:59Z', '10000-03-05T09:00:00Z', '-10000-03-05T09:00:00Z', '178956971-03-05T09:00:00Z'].each do |text|
    at = Time.iso8601(text)
    saved.update_columns(remind_at: at, reminded_at: at, status: 'done')
    scheduled.update_columns(send_at: at, sent_at: at, sent_message_id: message.id)
    Current.user = user
    Time.use_zone(zone) do
     queries = []
     observer = ->(*args) { p=args.last; queries << p[:sql] if !p[:cached] && p[:name] != 'SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
     rendered = nil
     ActiveSupport::Notifications.subscribed(observer, 'sql.active_record') do
      actual_saved = SavedItem.includes(message: [:room, :rich_text_body, :creator]).find(saved.id)
      actual_scheduled = ScheduledMessage.includes(:room, :thread, :sent_message).find(scheduled.id)
      renderer = ApplicationController.renderer.new(http_host: 'campfire.test', https: false)
      rendered = {
       saved: renderer.render(partial: 'saved_items/item', locals: {saved_item: actual_saved, status_filter: 'all'}),
       scheduled: renderer.render(partial: 'scheduled_messages/item', locals: {scheduled: actual_scheduled, stranded: false}),
       past: renderer.render(partial: 'scheduled_messages/past_item', locals: {scheduled: actual_scheduled})
      }
     end
     rows = {'saved_items'=>conn.select_all("SELECT * FROM saved_items WHERE id=#{saved.id}").to_a, 'scheduled_messages'=>conn.select_all("SELECT * FROM scheduled_messages WHERE id=#{scheduled.id}").to_a}
     cases << {zone:, input: text, rows:, html: rendered, reads: queries.length}
    end
   end
  end
  groups << group.slice('size','rows').merge(cases:)
 end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', groups:) + "\n")
puts "WS8bm2 wide HTML Rails: #{groups.sum { |g| g[:cases].length }} row/zone cases; 3 real partials each"
