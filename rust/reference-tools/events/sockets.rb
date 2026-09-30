require 'json'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
travel_to Time.utc(2026,2,10,12)
fixtures=Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures,Dir[fixtures.join('**/*.yml')].map{|p|p.delete_prefix("#{fixtures}/").delete_suffix('.yml')},{'twitter_posts'=>Twitter::Post,'twitter_post_references'=>Twitter::PostReference})
ActiveJob::Base.queue_adapter=:test
Rails.application.routes.default_url_options[:host]='example.com'
Rails.application.routes.default_url_options[:protocol]='http'
room=Room.find(ActiveRecord::FixtureSet.identify(:watercooler));david=User.find(ActiveRecord::FixtureSet.identify(:david));jason=User.find(ActiveRecord::FixtureSet.identify(:jason))
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream, payload, **options| frames << {stream:,payload:} }
out=[]
travel_to Time.utc(2026,3,2,16) do
  Current.user=david
  Event.connection.execute("UPDATE sqlite_sequence SET seq=8000000000 WHERE name='events'")
  Message.connection.execute("UPDATE sqlite_sequence SET seq=9000000000 WHERE name='messages'")
  ActivityItem.connection.execute("UPDATE sqlite_sequence SET seq=7000000000 WHERE name='activity_items'")
  event=room.events.create!(organizer:david,title:'Socket planning',starts_at:Time.utc(2026,3,2,16,10),time_zone:'UTC')
  message=event.referencing_messages.sole;message.update_column(:client_message_id,'ws14e-announcement')
  out << {kind:'invitation',frames:frames.select{|f|f[:stream]=="user_#{jason.id}_activity"}}
  frames.clear
  event.update!(title:'Changed <&>')
  out << {kind:'edit',frames:frames.select{|f|f[:payload].is_a?(String)&&f[:payload].include?('event_cards_message_ws14e-announcement')}}
  frames.clear
  event.respond!(jason,'maybe')
  out << {kind:'response',frames:frames.select{|f|f[:payload].is_a?(String)&&f[:payload].include?('event_cards_message_ws14e-announcement')}}
  frames.clear
  Event::ReminderDispatcher.dispatch_due!(now:Time.current)
  out << {kind:'reminder',frames:frames.select{|f|f[:stream]=="user_#{jason.id}_activity"||f[:payload].is_a?(String)&&f[:payload].include?('event_cards_message_ws14e-announcement')}}
  frames.clear
  event.cancel!(actor:david)
  out << {kind:'cancel',frames:frames.select{|f|f[:stream]=="user_#{jason.id}_activity"||f[:payload].is_a?(String)&&f[:payload].include?('event_cards_message_ws14e-announcement')}}
  frames.clear
  series=room.events.create!(organizer:david,title:'Two records',starts_at:Time.utc(2026,3,4,9),time_zone:'UTC',recurrence_rule:'weekly',recurrence_until:Date.new(2026,3,11))
  series.referencing_messages.sole.update_column(:client_message_id,'ws14e-series')
  ids=series.series_events.ids
  room.root_messages.create_with_attachment!(creator:david,client_message_id:'ws14e-shared',markdown_source:ids.map{|id|"http://example.com/rooms/#{room.id}/events/#{id}"}.join("\n"))
  out << {kind:'series_invitation',frames:frames.select{|f|f[:stream]=="user_#{jason.id}_activity"}}
  frames.clear
  series.update_with_scope!({title:'Shared changed'},scope:'this_and_following',actor:david)
  out << {kind:'two_records',frames:frames.select{|f|f[:payload].is_a?(String)&&f[:payload].include?('event_cards_message_ws14e')}}
  frames.clear
  Event.transaction do
    series.update!(description:'First')
    series.update!(description:'Second')
  end
  out << {kind:'one_record_twice',frames:frames.select{|f|f[:payload].is_a?(String)&&f[:payload].include?('event_cards_message_ws14e')}}
  frames.clear
  Event.transaction do
    series.update!(title:'Never delivered')
    series.destroy!
  end
  out << {kind:'update_then_destroy',frames:frames.select{|f|f[:stream]=="user_#{jason.id}_activity"||f[:payload].is_a?(String)&&f[:payload].include?('event_cards_message_ws14e')}}
  Current.reset
end
File.write('/rails/storage/db/event-sockets.json',JSON.pretty_generate(out)+"\n")
puts "Rails event sockets: #{out.map{|s|[s[:kind],s[:frames].size]}.to_h}"
