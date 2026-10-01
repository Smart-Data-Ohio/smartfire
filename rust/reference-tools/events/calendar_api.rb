require 'json'
require 'active_record/fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
fixtures = Rails.root.join('test/fixtures')
ActiveRecord::FixtureSet.create_fixtures(fixtures, Dir[fixtures.join('**/*.yml')].map { |p| p.delete_prefix("#{fixtures}/").delete_suffix('.yml') }, {'twitter_posts'=>Twitter::Post, 'twitter_post_references'=>Twitter::PostReference})
ActiveJob::Base.queue_adapter = :test
Rails.application.routes.default_url_options.merge!(host: 'example.com', protocol: 'http')
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, payload, **options| frames << {stream:, payload:} }
room = Room.find(ActiveRecord::FixtureSet.identify(:designers))
david = User.find(ActiveRecord::FixtureSet.identify(:david))
jason = User.find(ActiveRecord::FixtureSet.identify(:jason))
out = {respond: [], meet_link: []}
response_cases = [
  {name: 'singleton_new'},
  {name: 'singleton_changed', existing: 'going'},
  {name: 'singleton_unchanged', existing: 'maybe'},
  {name: 'head_new', series: true},
  {name: 'head_overwrites', series: true, existing: 'maybe', distinct: true},
  {name: 'follower_local', series: true, index: 1, existing: 'going'},
  {name: 'follower_future', series: true, index: 1, future: true, existing: 'going', distinct: true},
  {name: 'head_skips_cancelled', series: true, cancelled: 1},
  {name: 'last_has_no_future', series: true, index: 2, future: true},
  {name: 'cancelled_anchor', series: true, cancelled: 0},
  {name: 'bot', user: 'bender'},
  {name: 'nonmember', nonmember: true},
  {name: 'unknown_response', response: 'not-a-response'},
  {name: 'follower_failure', series: true, fail: true},
  {name: 'outer_rollback', series: true, rollback: true},
  {name: 'two_calls_one_transaction', series: true, twice: true},
  {name: 'two_calls_unchanged_first', series: true, existing: 'maybe', twice: true},
  {name: 'response_then_destroy', destroy: true}
]
meet_cases = [
  {name: 'new_link', link: 'https://meet.example.test/fixture'},
  {name: 'same_link', link: 'https://meet.example.test/fixture', existing: 'https://meet.example.test/fixture'},
  {name: 'changed_link', link: 'https://meet.example.test/changed', existing: 'https://meet.example.test/fixture'},
  {name: 'clear_link', link: nil, existing: 'https://meet.example.test/fixture'},
  {name: 'blank_link', link: ' '},
  {name: 'not_requested', link: nil, requested: false},
  {name: 'non_https_is_stored', link: 'http://meet.example.test/fixture'},
  {name: 'cancelled', link: 'https://meet.example.test/fixture', cancelled: true},
  {name: 'invalid_title', link: 'https://meet.example.test/fixture', invalid: 'title'},
  {name: 'invalid_end', link: 'https://meet.example.test/fixture', invalid: 'ends_at'},
  {name: 'invalid_zone', link: 'https://meet.example.test/fixture', invalid: 'time_zone'},
  {name: 'ineligible_organizer', link: 'https://meet.example.test/fixture', nonmember: true},
  {name: 'storage_failure', link: 'https://meet.example.test/fixture', fail: true},
  {name: 'outer_rollback', link: 'https://meet.example.test/fixture', rollback: true},
  {name: 'two_saves_one_transaction', link: 'https://meet.example.test/fixture', twice: true},
  {name: 'blank_twice', link: ' ', twice: true, second: ' '},
  {name: 'blank_then_link', link: ' ', twice: true},
  {name: 'link_then_blank', link: 'https://meet.example.test/fixture', twice: true, second: ' '}
]
begin
  (response_cases.map { |c| [:respond, c] } + meet_cases.map { |c| [:meet_link, c] }).each do |kind, spec|
    travel_to Time.utc(2026,9,22,12)
    event = room.events.create!(organizer: david, title: 'Planning session', starts_at: 10.minutes.from_now, time_zone: 'UTC', meet_link_requested: kind == :meet_link && spec.fetch(:requested, true), **(spec[:series] ? {recurrence_rule: 'weekly', recurrence_until: Date.new(2026,10,6)} : {}))
    rows = event.series_events.to_a
    actor = User.find(ActiveRecord::FixtureSet.identify(spec.fetch(:user, 'jason')))
    if spec[:existing] && kind == :respond
      rows.each { |r| r.respond!(actor, spec[:existing]) }
    end
    rows.last.respond!(actor, 'declined') if spec[:distinct]
    rows[spec[:cancelled]].update_column(:cancelled_at, Time.current) if kind == :respond && spec[:cancelled]
    event.update_column(:meet_link, spec[:existing]) if kind == :meet_link && spec.key?(:existing)
    event.update_column(:cancelled_at, Time.current) if kind == :meet_link && spec[:cancelled]
    case spec[:invalid]
    when 'title' then event.update_column(:title, '')
    when 'ends_at' then event.update_column(:ends_at, event.starts_at)
    when 'time_zone' then event.update_column(:time_zone, 'Mars/Olympus')
    end
    removed_user = kind == :respond ? actor : david
    membership = room.memberships.find_by(user: removed_user)
    membership.delete if spec[:nonmember]
    if kind == :meet_link
      event.referencing_messages.sole.update_column(:client_message_id, "api-#{spec[:name]}-announcement")
      room.root_messages.create_with_attachment!(creator: david, client_message_id: "api-#{spec[:name]}-extra", markdown_source: "http://example.com/rooms/#{room.id}/events/#{event.id}") unless spec[:nonmember]
    end
    if spec[:fail]
      sql = kind == :respond ? "BEFORE INSERT ON event_attendances WHEN NEW.event_id=#{rows[1].id} AND NEW.user_id=#{actor.id}" : 'BEFORE UPDATE OF meet_link ON events'
      Event.connection.execute("CREATE TEMP TRIGGER reject_calendar_api #{sql} BEGIN SELECT RAISE(ABORT, 'calendar api unavailable'); END")
    end
    frames.clear
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    result = 'ok'
    errors = []
    before_commit = nil
    travel_to Time.utc(2026,9,22,12,1)
    begin
      begin
        Event.transaction do
          if kind == :respond
            selected = rows.fetch(spec.fetch(:index, 0))
            selected.respond!(actor, spec.fetch(:response, 'maybe'), apply_to_future: spec.fetch(:future, false))
            selected.respond!(actor, 'declined', apply_to_future: true) if spec[:twice]
            selected.destroy! if spec[:destroy]
          else
            event.reload.update!(meet_link: spec[:link])
            event.update!(meet_link: spec.fetch(:second, 'https://meet.example.test/second')) if spec[:twice]
          end
          before_commit = {jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.size, frames: frames.size}
          raise 'outer rollback' if spec[:rollback]
        end
      rescue => e
        result = e.class.name
        errors = e.record.errors.full_messages if e.is_a?(ActiveRecord::RecordInvalid)
      end
    end
    Event.connection.execute('DROP TRIGGER reject_calendar_api') if spec[:fail]
    room.memberships.create!(user: removed_user, involvement: membership.involvement) if spec[:nonmember]
    jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job].name.start_with?('Calendar::') }.map { |j| [j[:job].name, j[:args].map { |a| rows.map(&:id).include?(a) ? "event-#{rows.map(&:id).index(a)}" : a }] }
    cards = frames.filter_map do |f|
      next unless f[:payload].is_a?(String) && f[:payload].include?('event_cards_message_api-')
      {stream: f[:stream], target: f[:payload][/target="([^"]+)"/, 1], action: f[:payload][/action="([^"]+)"/, 1], maintain_scroll: f[:payload].include?('maintain_scroll="true"')}
    end
    state = if kind == :respond
      rows.map { |r| a = r.attendances.find_by(user: actor); a && {response: a.response, updated_at: a.updated_at.utc.iso8601(6)} }
    else
      event.reload.slice('meet_link', 'updated_at').merge('needs_meet_link' => event.needs_meet_link?).tap { |s| s['updated_at'] = event.updated_at.utc.iso8601(6) }
    end
    out[kind] << {spec:, result:, errors:, before_commit:, state:, jobs:, cards:}
  end
end
File.write('/rails/storage/db/event-calendar-api.json', JSON.pretty_generate(out)+"\n")
puts "Rails event Calendar APIs: #{out[:respond].size} response states, #{out[:meet_link].size} internal Meet-link states"
