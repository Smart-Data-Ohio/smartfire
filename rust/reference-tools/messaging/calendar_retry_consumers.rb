# Real registered Calendar jobs: remote failures, retries, exhaustion and complete rows.
require 'json'
require 'net/http'
require_relative 'oracle-database'
ActiveJob::Base.queue_adapter = :test
Kernel.define_singleton_method(:rand) { 0.0 }
Rails.application.routes.default_url_options.merge!(host: 'campfire.test', protocol: 'http')
FIXTURE_TOKEN = 'fixture-calendar-access'
class CalendarRetryHTTP
  def initialize(host) = (@host = host)
  %w[get delete post put].each do |verb|
    define_method(verb) do |path, *args|
      body, headers = args.length == 1 ? [nil, args[0]] : args
      route = Thread.current.fetch(:calendar_retry_routes).shift
      raise "unlisted exchange #{verb} #{path}" unless @host == 'www.googleapis.com' && route && route[:method] == verb.upcase && route[:path] == path
      raise 'wrong fixture credential' unless headers['Authorization'] == ['Bearer', FIXTURE_TOKEN].join(' ')
      Thread.current[:calendar_retry_calls] << {method: verb.upcase, path:, body: body && JSON.parse(body)}
      if route[:failure]
        raise({'open_timeout' => Net::OpenTimeout, 'read_timeout' => Net::ReadTimeout, 'write_timeout' => Net::WriteTimeout}.fetch(route[:failure]))
      end
      response = Net::HTTPResponse::CODE_TO_OBJ.fetch(route[:status].to_s).new('1.1', route[:status].to_s, 'fixture')
      response.instance_variable_set(:@read, true)
      response.body = route.fetch(:body).to_json
      response
    end
  end
end
module CalendarRetryNetwork
  def start(host, port, **options)
    raise 'external network prohibited' unless host == 'www.googleapis.com' && port == 443 && options[:use_ssl]
    yield CalendarRetryHTTP.new(host)
  end
end
Net::HTTP.singleton_class.prepend(CalendarRetryNetwork)
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, html, **| frames << {stream:, html:} }
conn = ActiveRecord::Base.connection
source = JSON.parse(File.read('/work/vectors/messaging/older_calendar_execution.json'))
user = User.find(127326141)
Current.user = user
matrix = {
  'rate_recover' => [429, 429, 200],
  'rate_exhaust' => [429] * 8,
  'transport_recover' => %w[open_timeout read_timeout write_timeout] + [200],
  'transport_exhaust' => ['read_timeout'] * 8,
  'mixed_recover' => [429, 'open_timeout', 503, 200]
}
groups = []
source.fetch('groups').each do |group|
  conn = ActiveRecord::Base.connection
  EventCalendarEntry.delete_all
  group.fetch('rows').each do |table, rows|
    rows.each do |row|
      conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |k| conn.quote_column_name(k) }.join(',')}) VALUES (#{row.values.map { |v| conn.quote(v) }.join(',')})")
    end
  end
  account = GoogleAccount.find_or_initialize_by(user:)
  account.update!(email: 'fixture@calendar.test', access_token: FIXTURE_TOKEN, refresh_token: 'fixture-calendar-refresh', access_token_expires_at: 1.hour.from_now, scopes: Google::Client::CALENDAR_SCOPE, disconnected_reason: nil)
  reset = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
  cases = []
  %w[inbound upsert remove].each do |action|
    matrix.each do |name, responses|
      reset.call do
        conn = ActiveRecord::Base.connection
        event = Event.find(group.fetch('event_id'))
        copy = EventCalendarEntry.find(group.fetch('entry_id'))
        EventAttendance.where(event:, user:).update_all(response: 'declined') if action == 'remove'
        klass = action == 'inbound' ? Calendar::InboundSyncJob : Calendar::SyncEntryJob
        arguments = action == 'inbound' ? [user.id] : [event.id, user.id]
        path = "/calendar/v3/calendars/primary/events/#{copy.google_event_id}"
        method = {'inbound' => 'GET', 'upsert' => 'PUT', 'remove' => 'DELETE'}.fetch(action)
        job = klass.new(*arguments)
        steps = []
        responses.each do |answer|
          route = {method:, path:, body: action == 'inbound' ? {status: 'confirmed'} : {id: copy.google_event_id}}
          answer.is_a?(Integer) ? route[:status] = answer : route[:failure] = answer
          Thread.current[:calendar_retry_routes] = [route]
          Thread.current[:calendar_retry_calls] = []
          frames.clear
          ActiveJob::Base.queue_adapter.enqueued_jobs.clear
          # Freeze jitter, not the retry policy. Rust asserts its production jitter bounds.
          error = nil
          job.define_singleton_method(:rescue_with_handler) do |failure|
            error = {class: failure.class.name, message: failure.message}
            super(failure)
          end
          queries = []
          observer = ->(*args) { p = args.last; queries << p[:sql] if !p[:cached] && p[:name] != 'SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
          ActiveSupport::Notifications.subscribed(observer, 'sql.active_record') do
            begin
              job.perform_now
            rescue Google::Client::Unavailable => e
              error = {class: e.class.name, message: e.message}
            end
          end
          raise 'unused network route' unless Thread.current[:calendar_retry_routes].empty?
          queued = ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == klass }
          raise 'multiple retries' if queued.length > 1
          steps << {
            route:, calls: Thread.current[:calendar_retry_calls].dup, error:,
            queue: {retry: queued.any?, executions: job.executions, counts: job.exception_executions.to_h.dup, wait: queued.first && queued.first.fetch(:at) - Time.current.to_f},
            entry: conn.select_one("SELECT * FROM event_calendar_entries WHERE id=#{copy.id}"),
            event: conn.select_one("SELECT * FROM events WHERE id=#{event.id}"),
            attendance: conn.select_all("SELECT * FROM event_attendances WHERE event_id=#{event.id} ORDER BY id").to_a,
            frames: frames.dup, reads: queries.length
          }
          break if queued.empty?
          job = klass.deserialize(queued.first)
        end
        cases << {name: "#{action}_#{name}", action:, steps:}
      end
    end
  end
  groups << group.slice('size', 'event_id', 'entry_id', 'thread_id', 'rows').merge(cases:)
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], groups:) + "\n")
puts "WS8bm2 Calendar retry Rails: #{groups.sum { |g| g[:cases].length }} cases; #{groups.sum { |g| g[:cases].sum { |c| c[:steps].length } }} real job executions; complete persisted rows, calls, counters, retry delays and broadcasts"
