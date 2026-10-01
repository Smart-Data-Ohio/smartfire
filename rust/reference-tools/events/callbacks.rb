require "json"
require "active_record/fixtures"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
travel_to Time.utc(2026,9,22,12) do
  fixtures = Rails.root.join("test/fixtures")
  ActiveRecord::FixtureSet.create_fixtures(fixtures,
    Dir[fixtures.join("**/*.yml")].map { |path| path.delete_prefix("#{fixtures}/").delete_suffix(".yml") },
    { "twitter_posts" => Twitter::Post, "twitter_post_references" => Twitter::PostReference })
  ActiveJob::Base.queue_adapter = :test
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  event=Room.find(ActiveRecord::FixtureSet.identify(:designers)).events.create!(organizer: User.find(ActiveRecord::FixtureSet.identify(:david)),title:"Planning session", starts_at:10.minutes.from_now,time_zone:"UTC",recurrence_rule:"weekly",recurrence_until:Date.new(2026,10,6),meet_link_requested:true)
  ids=event.series_events.ids
  callbacks = ActiveJob::Base.queue_adapter.enqueued_jobs.select {|j|j[:job].name.start_with?("Calendar::")}.map {|j|[j[:job].name,j[:args].map {|a|ids.include?(a) ? "event-#{ids.index(a)}" : a}]}
  File.write("/rails/storage/db/event-create-callbacks.json", JSON.pretty_generate(callbacks) + "\n")
  puts "Rails event create callbacks: #{callbacks.length} jobs"
end
