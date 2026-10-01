require 'json'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
fixtures = Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures, Dir[fixtures.join('**/*.yml')].map { |p| p.delete_prefix("#{fixtures}/").delete_suffix('.yml') }, {'twitter_posts'=>Twitter::Post, 'twitter_post_references'=>Twitter::PostReference})
ActiveJob::Base.queue_adapter = :test
david = User.find(ActiveRecord::FixtureSet.identify(:david))
room = Room.find(ActiveRecord::FixtureSet.identify(:designers))
out = {urls: [], rejected: {}, nil_starts: [], production_500: File.read(Rails.root.join('public/500.html'))}
travel_to Time.utc(2026,9,22,12) do
  ['https://events.example.test', 'http://events.example.test:53200', 'https://events.example.test:8443'].each do |origin|
    uri = URI.parse(origin)
    Rails.application.routes.default_url_options.merge!(host: uri.host, protocol: uri.scheme, port: uri.port)
    event = room.events.create!(organizer: david, title: 'Origin <&>', time_zone: 'UTC', starts_at: Time.utc(2026,10,5,9))
    out[:urls] << {origin:, suffix: "/rooms/#{room.id}/events/#{event.id}", announcement: Message.order(:id).last.markdown_source}
  end
  board = Rooms::Board.create!(name: 'Rejected event board', creator: david)
  board.memberships.find_or_create_by!(user: david) { |m| m.involvement = 'everything' }
  board.memberships.find_or_create_by!(user: User.find(ActiveRecord::FixtureSet.identify(:jason))) { |m| m.involvement = 'everything' }
  before = [Event.count, EventAttendance.count, ActivityItem.where(source_type: 'Event').count, Message.count]
  begin
    board.events.create!(organizer: david, title: 'Rejected announcement', time_zone: 'UTC', starts_at: Time.utc(2026,10,5,9))
  rescue => e
    out[:rejected] = {exception: e.class.name, delta: [Event.count, EventAttendance.count, ActivityItem.where(source_type: 'Event').count, Message.count].zip(before).map { |a,b|a-b }}
  end
  head = room.events.create!(organizer: david, title: 'Nil start', time_zone: 'UTC', starts_at: Time.utc(2026,10,5,9), recurrence_rule: 'weekly', recurrence_until: Date.new(2026,10,19))
  head.series_events.each_with_index do |event,index|
    ['this_event','this_and_following','all'].each do |scope|
      before = Event.order(:id).map(&:attributes)
      begin
        event.update_with_scope!({starts_at: nil}, scope:, actor: david)
        result = 'success'
      rescue => e
        result = e.class.name
      end
      out[:nil_starts] << {index:,scope:,exception:result,unchanged: before == Event.order(:id).map(&:attributes)}
      event.reload
    end
  end
end
File.write('/rails/storage/db/event-edges.json', JSON.pretty_generate(out)+"\n")
puts "Rails event callback edges: #{out[:urls].size} origins, rejected announcement #{out[:rejected]}, #{out[:nil_starts].size} nil starts"
