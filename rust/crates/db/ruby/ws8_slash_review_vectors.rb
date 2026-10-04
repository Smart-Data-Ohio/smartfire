# Astra review corpus, regenerated from Rails at parity/reference.sha. Expectations are computed, never supplied by the input cases.
require "json"
load "/tools/load_fixtures.rb"
ActiveJob::Base.queue_adapter = :test
extend ActiveSupport::Testing::TimeHelpers

def stamp(value)
  value&.utc&.strftime("%Y-%m-%d %H:%M:%S.%6N")&.sub(/\.000000\z/, "")
end

def split_stamp(pair, leading:)
  return nil unless pair
  leading ? [stamp(pair[0]), pair[1]] : [pair[0], stamp(pair[1])]
end

texts = ["in 1 day", "in 2 days", "in 1 week", "in 5 weeks", "tomorrow 1:30am", "tomorrow 2:30am", "today at 1:30am", "at 2:30am", "sunday 1:30am", "next sunday 1:30am", "2026-10-01t15:00", "in 20 minutes check", "tomorrow 9am check", "in 20 minutesé", "2026-10-01 24:00", "2026-10-01 23:59:60", "2026-12-31 24:00", "2026-02-29 09:00", "Sept 30 5pm", "Oct 1 2026 17:00 EST", "2026-10-01 17:00 EDT", "17:00 EDT", "5 pm EST", "2026-10", "Oct 2026", "2026-11-01 01:30", "2026-10-25 01:30", "2026-10-24 23:30", "2026-10-25 00:30"]
clocks = ["2026-01-30 12:00:00", "2026-01-31 12:00:00", "2026-02-28 12:00:00", "2026-12-31 12:00:00", "2028-02-28 12:00:00", "2026-11-01 06:30:00", "2026-10-25 01:30:00", "2026-10-24 12:00:00", "2026-03-07 17:00:00", "2026-10-31 12:00:00"]
zones = ["UTC", "America/New_York", "Europe/Dublin", "Europe/London", "Australia/Lord_Howe", "Pacific/Apia", "America/Nuuk", "America/Havana", "Asia/Gaza"]
parsing = clocks.product(zones, texts).map do |clock, zone, text|
  now = Time.iso8601(clock.tr(" ", "T") + "Z")
  travel_to now
  {now: clock, zone: zone, text: text, parse: stamp(SlashCommands::TimeParser.parse(text, zone: zone, now: now)), leading: split_stamp(SlashCommands::TimeParser.split_leading_time(text, zone: zone, now: now), leading: true), trailing: split_stamp(SlashCommands::TimeParser.split_trailing_time(text, zone: zone, now: now), leading: false)}
end
registry = SlashCommands::Registry.all.map { |c| {name: c.name, description: c.description, arg_hint: c.arg_hint, takes_arguments: c.takes_arguments, root: c.permission.call(nil,nil,nil), thread: c.permission.call(nil,nil,Object.new)} }
command_texts = ["/poll", "/ME waves\nhello", " /shrug ", "//poll", "/x_2-hi", "/é", "/2hi", "/me\t waves", "/me\u00a0waves", "hello /poll", "just text", "/poll!", "/me\nhello"]
commands = command_texts.map { |text| {text: text, recognized: SlashCommands::Dispatcher.command_text?(text)} }

# Capture the transport seam without rendering user-dependent HTML or making network calls.
$calls = []
%i[broadcast_update_to broadcast_append_to broadcast_replace_to].each do |method|
  Turbo::StreamsChannel.define_singleton_method(method) do |*streams, **options|
    $calls << {method: method, streams: streams.flatten.map { |s| s.respond_to?(:to_global_id) ? s.to_global_id.to_s : s.to_s }, target: options[:target].is_a?(Array) ? ActionView::RecordIdentifier.dom_id(options[:target][0],options[:target][1]) : options[:target], partial: options[:partial]}
  end
end
ActionCable.server.define_singleton_method(:broadcast) { |stream, payload| $calls << {method: "cable", stream: stream, payload: payload} }
def copy_db(destination, source)
  backup = SQLite3::Backup.new(destination, "main", source, "main")
  backup.step(-1)
  backup.finish
end
baseline = SQLite3::Database.new(":memory:")
copy_db(baseline, ActiveRecord::Base.connection.raw_connection)
rows = []
cases = JSON.parse(<<~JSON, symbolize_names: true)
[
  {
    "text": "/status 🚂 Train",
    "place": "root",
    "now": "2026-10-24 12:00:00",
    "zone": "America/Nuuk"
  },
  {
    "text": "/ooo today",
    "place": "root",
    "now": "2026-10-24 12:00:00",
    "zone": "America/Nuuk"
  },
  {
    "text": "/status 🚂 Train",
    "place": "root",
    "now": "2026-10-31 12:00:00",
    "zone": "America/Havana"
  },
  {
    "text": "/remind 2026-10-01 24:00 check",
    "place": "root",
    "now": "2026-09-23 12:00:00",
    "zone": "UTC"
  },
  {
    "text": "/remind in 20 minutes check",
    "place": "root",
    "now": "2026-09-23 12:00:00",
    "zone": "UTC"
  },
  {
    "text": "/remind 2026-10-01t15:00 check",
    "place": "root",
    "now": "2026-09-23 12:00:00",
    "zone": "UTC"
  },
  {
    "text": "/dnd until 17:00 EDT",
    "place": "root",
    "now": "2026-09-23 12:00:00",
    "zone": "America/New_York"
  },
  {
    "text": "/status",
    "place": "root",
    "now": "2026-09-23 12:00:00",
    "zone": "UTC",
    "initial": {
      "time_zone": "bad"
    }
  },
  {
    "text": "/remind tomorrow 1:30am check",
    "place": "root",
    "now": "2026-10-24 12:00:00",
    "zone": "Europe/Dublin"
  },
  {
    "text": "/remind sunday 1:30am check",
    "place": "root",
    "now": "2026-10-24 12:00:00",
    "zone": "Europe/Dublin"
  },
  {
    "text": "/dnd until 17:00 EST",
    "place": "root",
    "now": "2026-09-23 12:00:00",
    "zone": "America/New_York"
  },
  {
    "text": "/remind 2026-12-31 24:00 check",
    "place": "root",
    "now": "2026-12-31 12:00:00",
    "zone": "UTC"
  }
]
JSON
cases.each do |input|
  text = input[:text]; place = input[:place]
  begin
    copy_db(ActiveRecord::Base.connection.raw_connection, baseline)
    ActiveRecord::Base.connection.clear_query_cache
    travel_to Time.zone.parse(input[:now] || "2026-09-23 12:00:00")
    user = User.find(ActiveRecord::FixtureSet.identify("david")); user.update!(time_zone: input[:zone] || "America/New_York")
    user.update_columns(input[:initial]) if input[:initial]
    Calendar::MeetingCache.create!(user: user, fetched_at: Time.current, ooo_intervals: input[:intervals]) if input[:intervals]
    room = Room.find(ActiveRecord::FixtureSet.identify("watercooler"))
    if place == "board"
      room.update!(type: "Rooms::Board"); room = Room.find(room.id)
    end
    thread = ChannelThread.create!(room: room, creator: user, name: "Slash test") if %w[thread locked].include?(place)
    thread.update!(locked_at: Time.current) if place == "locked"
    $calls = []; ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    Huddle.define_singleton_method(:configured?) { input[:huddles] || false }
    result = nil; exception = nil
    begin
      result = SlashCommands::Dispatcher.dispatch(user: user, room: room, thread: thread, text: text)
    rescue ActiveRecord::RecordInvalid, ArgumentError => error
      exception = {exception: error.class.name, message: error.is_a?(ActiveRecord::RecordInvalid) ? error.record.errors.full_messages.to_sentence : error.message}
    end
    message = Message.find_by(id: result&.payload&.dig(:message_id))
    user.reload
    selected = %w[custom_status_emoji custom_status_text custom_status_expires_at dnd_enabled dnd_until ooo_until ooo_note ooo_broadcast updated_at]
    state = selected.to_h { |key| v=user.public_send(key); [key, v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? stamp(v) : v] }
    message_state = message && message.attributes.slice("action","system_note","streaming","markdown_source","created_at","updated_at").transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? stamp(v) : v }
    saved = message && SavedItem.find_by(user: user,message: message)
    rows << {text: text, place: place, now: input[:now] || "2026-09-23 12:00:00", zone: input[:zone] || "America/New_York", initial: input[:initial] || {}, intervals: input[:intervals], huddles: input[:huddles] || false, stateful: input[:stateful] || false, result: exception || {kind: result.kind, message: result.message, notice: result.notice, url: result.url}, user: state, message: message_state, saved: saved && {remind_at: stamp(saved.remind_at), status: saved.status, reminded_at: stamp(saved.reminded_at), created_at: stamp(saved.created_at), updated_at: stamp(saved.updated_at)}, rich_text: message && {name: message.rich_text_body.name, body: message.body.body.to_html, record_type: message.rich_text_body.record_type, created_at: stamp(message.rich_text_body.created_at), updated_at: stamp(message.rich_text_body.updated_at)}, index: message && ActiveRecord::Base.connection.select_value("SELECT body FROM message_search_index WHERE rowid=#{message.id}"), thread: thread && thread.reload.attributes.slice("messages_count", "closed_at", "last_activity_at", "updated_at").transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? stamp(v) : v }, jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j| j[:job].name }.sort, broadcasts: $calls}

  end
end
# Filled below, after generating Ruby's timezone vocabulary.
output = {pin: ENV.fetch("PARITY_REFERENCE_SHA"), parsing: parsing, registry: registry, commands: commands, rows: rows}
File.write(ARGV.fetch(1), JSON.pretty_generate(ActiveSupport::TimeZone::MAPPING)+"\n")
puts "WS8 slash review Rails vectors: #{parsing.size} parsing, #{registry.size} registry, #{commands.size} recognition, #{rows.size} dispatch/row/callback cases"

# Discover Ruby Date's abbreviation vocabulary from the installed pinned gem, then
# ask its parser for each offset instead of copying a hand-written offset table.
zone_source = File.join(Gem::Specification.find_by_name("date").full_gem_path, "ext/date/zonetab.list")
zone_names = File.read(zone_source).split("%%")[1].lines.filter_map { |line| line.split(",", 2).first&.strip }.reject(&:empty?)
offsets = zone_names.flat_map { |name| [name, "#{name} dst", "#{name} standard time", "#{name} daylight time"] }.to_h { |name| [name, Date._parse("17:00 #{name}")[:offset]] }.compact
File.write(ARGV.fetch(2), JSON.pretty_generate(offsets) + "\n")

# Additional coverage checks the offset vocabulary through the actual parser.
# Keep the reviewer's 2610 cases in their original field and order.
supplemental_texts = offsets.keys.map { |name| "17:00 #{name}" } + [
  "2026-10-01 24:01", "2026-10-01 24:00:01", "2026-10-01 24:00:00.1",
  "2026-10-01 23:59:60.1", "17:00 +0530", "17:00 -0430", "17:00 XYZ"
]
output[:supplemental_parsing] = ["UTC", "America/New_York"].product(supplemental_texts).map do |zone, text|
  clock = "2026-09-23 12:00:00"
  now = Time.iso8601(clock.tr(" ", "T") + "Z")
  travel_to now
  {now: clock, zone: zone, text: text, parse: stamp(SlashCommands::TimeParser.parse(text, zone: zone, now: now)), leading: split_stamp(SlashCommands::TimeParser.split_leading_time(text, zone: zone, now: now), leading: true), trailing: split_stamp(SlashCommands::TimeParser.split_trailing_time(text, zone: zone, now: now), leading: false)}
end
# Period identity, rather than merely the same UTC offset in a different year.
["in 364 days", "in 52 weeks"].each do |text|
  clock = "2025-10-26 00:30:00"
  now = Time.iso8601(clock.tr(" ", "T") + "Z")
  travel_to now
  zone = "Europe/Dublin"
  output[:supplemental_parsing] << {now: clock, zone: zone, text: text, parse: stamp(SlashCommands::TimeParser.parse(text, zone: zone, now: now)), leading: split_stamp(SlashCommands::TimeParser.split_leading_time(text, zone: zone, now: now), leading: true), trailing: split_stamp(SlashCommands::TimeParser.split_trailing_time(text, zone: zone, now: now), leading: false)}
end
File.write(ARGV.fetch(0), JSON.pretty_generate(output) + "\n")
puts "WS8 slash supplemental Rails vectors: #{output[:supplemental_parsing].size} parsing cases"
