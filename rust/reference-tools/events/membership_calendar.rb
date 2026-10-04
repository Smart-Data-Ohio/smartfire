require 'json'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers

ActiveRecord::Base.logger = nil
ActiveJob::Base.queue_adapter = :test
Rails.application.routes.default_url_options.merge!(host: 'example.com', protocol: 'http')
ActionCable.server.define_singleton_method(:broadcast) { |*| }
fixtures = Rails.root.join('test/fixtures')
names = Dir[fixtures.join('**/*.yml')].map { |p| p.delete_prefix("#{fixtures}/").delete_suffix('.yml') }
cases = %w[matching_entries no_entries other_room other_user removed_later added_later rollback]
rows = cases.map do |name|
  travel_to Time.utc(2026, 9, 22, 12)
  EventCalendarEntry.delete_all
  ActiveRecord::FixtureSet.reset_cache
  ActiveRecord::FixtureSet.create_fixtures(fixtures, names, {'twitter_posts' => Twitter::Post, 'twitter_post_references' => Twitter::PostReference})
  user = User.find(ActiveRecord::FixtureSet.identify(:david))
  event = Event.find(ActiveRecord::FixtureSet.identify(:launch_party))
  other_event = Event.find(ActiveRecord::FixtureSet.identify(:watercooler_sync))
  membership = Membership.find_by!(user: user, room: event.room)
  entry = nil
  unless %w[no_entries added_later].include?(name)
    entry = EventCalendarEntry.create!(event: name == 'other_room' ? other_event : event,
      user: name == 'other_user' ? User.find(ActiveRecord::FixtureSet.identify(:jason)) : user,
      google_event_id: 'disposable-calendar-copy')
  end
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  before_commit = nil
  Event.transaction do
    membership.destroy!
    entry.delete if name == 'removed_later'
    EventCalendarEntry.create!(event: event, user: user, google_event_id: 'disposable-calendar-copy') if name == 'added_later'
    before_commit = ActiveJob::Base.queue_adapter.enqueued_jobs.count { |j| j[:job] == Calendar::SyncEntryJob }
    raise ActiveRecord::Rollback if name == 'rollback'
  end
  jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == Calendar::SyncEntryJob }.map { |j| [j[:job].name, j[:args]] }
  {name: name, before_commit: before_commit, membership_exists: Membership.exists?(membership.id), jobs: jobs}
end
out = {reference: 'd7c7de92', source: 'app/models/membership.rb:282', rows: rows}
File.write('/rails/storage/db/membership-calendar.json', JSON.pretty_generate(out) + "\n")
puts "Pinned Rails membership Calendar callback: #{rows.size} cases; no Google HTTP"
