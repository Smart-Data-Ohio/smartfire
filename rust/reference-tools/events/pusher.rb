require "json"
require "active_record/fixtures"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
fixtures=Rails.root.join("test/fixtures")
ActiveRecord::FixtureSet.create_fixtures(fixtures,Dir[fixtures.join("**/*.yml")].map {|p|p.delete_prefix("#{fixtures}/").delete_suffix(".yml")},{"twitter_posts"=>Twitter::Post,"twitter_post_references"=>Twitter::PostReference})
ActiveJob::Base.queue_adapter=:test
vectors=[]
travel_to Time.utc(2026,9,22,12) do
  [-301,-300,0,30,89,90,900].each do |offset|
    ["designers","david_and_jason"].each do |name|
      room=Room.find(ActiveRecord::FixtureSet.identify(name))
      event=room.events.create!(organizer:User.find(ActiveRecord::FixtureSet.identify(:david)),title:"Planning session",time_zone:"UTC",starts_at:Time.current+offset)
      pusher=Event::ReminderPusher.new(event:)
      stale=Event::ReminderPusher.stale?(event,now:Time.current)
      payload=pusher.send(:build_payload,Time.current)
      payload[:path]=payload[:path].sub(/\/events\/\d+\z/,"/events/EVENT")
      payload[:tag]="event-EVENT"
      vectors<<{room:name,offset:,stale:,payload:stale ? nil : payload}
    end
  end
end
File.write("/rails/storage/db/event-pusher.json",JSON.pretty_generate(vectors)+"\n")
puts "Rails reminder push source vectors: #{vectors.length} cases"
