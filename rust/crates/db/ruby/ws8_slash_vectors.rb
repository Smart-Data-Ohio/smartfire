# Rails oracle at fec615be. Expectations are computed, never supplied by the input cases.
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

texts = ["", "sometime-ish", "in zero minutes", "in 0 minutes", "in 1 min", "in 20 minutes review the deploy", "in 3 hrs", "in 2 days", "in 1 week", "in 2 weeks", "tomorrow", "tomorrow at 2:30am", "tomorrow 2:15am", "tomorrow 9:30pm", "today at 3pm", "today at 1am", "at 15:00", "at 0am", "at 24:00", "at 12:60", "at 12pm", "at 12am", "friday", "friday 5pm", "next friday", "next sunday 2:30am", "sunday 1:30am", "wednesday 5pm", "mondayland", "tomorrowish", "2026-03-08 02:30", "2026-11-01 01:30", "2026-02-30 15:00", "2026-10-01T15:00:05", "2026-10-01", "5pm", "17:30", "Oct 5", "1 Oct 2026 15:00", "10/01/2026 15:00", "2026/10/01 15:00", "03-10-2026 15:00", "October 5, 2026", "3:05:20pm", "Thu, 01 Oct 2026 15:00:00 +0530", "2026-10-25 01:30", "review the deploy friday", "Launch party friday 5pm", "Friday", "Party", "  tomorrow 9am  hello\nworld  ", "next friday 9am trailing"]
clocks = ["2026-09-23 12:00:00", "2026-03-07 17:00:00", "2026-03-08 06:59:00", "2026-11-01 04:30:00", "2026-11-01 06:30:00", "2026-10-03 15:00:00", "2011-12-29 12:00:00", "1985-12-31 12:00:00"]
zones = ["UTC", "America/New_York", "Eastern Time (US & Canada)", "Asia/Kathmandu", "Pacific/Apia", "Australia/Lord_Howe", "Invalid/Zone", "Europe/Dublin"]
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
inputs = ["/poll", "/poll ignored", "/wat", "//poll", "/huddle", "/event", "/event Launch party friday 5pm", "/event Launch party", "/event ~!*()+ café friday 5pm", "/event Retro 2026-09-01 15:00", "/shrug", "/shrug ship it", "/me waves", "/me", "/remind", "/remind sometime review", "/remind in 20 minutes", "/remind in 20 minutes review deploy", "/remind 2026-09-01 15:00 old", "/status 🚂 On a train", "/status", "/status abcdefghi Too long", "/status 🚂 " + "x"*101, "/dnd", "/dnd off", "/dnd ON", "/dnd 2h", "/dnd 3d", "/dnd 0m", "/dnd until 5pm", "/dnd until at 5pm", "/dnd eventually", "/ooo 2h Back soon", "/ooo 1 week", "/ooo friday 5pm Wrapping up", "/ooo tomorrow Back soon", "/ooo today", "/ooo friday", "/ooo next wednesday", "/ooo 2026-10-05", "/ooo oct 8 Back soon", "/ooo sep 1", "/ooo feb 30", "/ooo feb 29", "/ooo 3d", "/ooo 0m", "/ooo tomorrow " + "x"*141, "/ooo off", "/play rimshot", "/play", "/SHRUG hi\nthere"]
stateful = [
  {text: "/dnd", initial: {dnd_enabled: true}},
  {text: "/dnd", initial: {dnd_enabled: true, dnd_until: "2026-09-23 11:59:59"}},
  {text: "/dnd on", initial: {dnd_enabled: true, updated_at: "2026-09-23 11:00:00"}},
  {text: "/ooo off", initial: {ooo_calendar_enabled: true, ooo_until: "2026-09-24 12:00:00"}, intervals: [["2026-09-23T11:55:00Z", "2026-09-26T12:00:00Z"]]},
  {text: "/ooo 2h note", initial: {ooo_calendar_enabled: true}, intervals: [["2026-09-23T11:55:00Z", "2026-09-26T12:00:00Z"]]},
  {text: "/ooo off", initial: {ooo_calendar_enabled: true}, intervals: [["bad", "bad"], ["2026-09-22T12:00:00Z", "2026-09-23T12:00:00Z"]]},
  {text: "/ooo off", initial: {ooo_calendar_enabled: true}, intervals: [["2026-09-23T12:00:00Z", "2026-09-24T12:00:00Z"]]},
  {text: "/ooo off", initial: {ooo_calendar_enabled: false}, intervals: [["2026-09-23T12:00:00Z", "2026-09-24T12:00:00Z"]]},
  {text: "/status 🚂 Train", initial: {theme: "bad", text_size: "bad"}},
  {text: "/status 🚂 Train", initial: {presence_setting: "bad"}},
  {text: "/status 🚂 Train", initial: {quiet_hours_enabled: true}},
  {text: "/status 🚂 Train", initial: {quiet_hours_start_minute: -1, quiet_hours_end_minute: 1440}},
  {text: "/status 🚂 Train", initial: {time_zone: "bad"}},
  {text: "/status 🚂 Train", initial: {voice_mode: "bad", push_to_talk_key: "x"*21}},
  {text: "/status 🚂 Train", initial: {inbox_preferences: {agent_work: "bad"}}},
  {text: "/ooo tomorrow note", initial: {custom_status_emoji: "x"*9, custom_status_text: "x"*101}},
  {text: "/dnd on", initial: {theme: "bad"}},
  {text: "/status 🚂 Train", zone: "Asia/Kathmandu", now: "2026-09-23 18:30:00"},
  {text: "/status 🚂 Train", zone: "America/New_York", now: "2026-03-08 06:00:00"},
  {text: "/ooo 3d", zone: "America/New_York", now: "2026-03-07 17:00:00"},
  {text: "/remind in 2 days review", zone: "America/New_York", now: "2026-03-07 17:00:00"},
  {text: "/ooo tomorrow note", zone: "Pacific/Apia", now: "2011-12-29 12:00:00"},
  {text: "/huddle", huddles: true}
]
cases = inputs.product(["root", "thread", "locked", "board"]).map { |text,place| {text: text,place: place} } + stateful.map { |c| c.merge(place: "root", stateful: true) }
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
File.write(ARGV.fetch(0), JSON.pretty_generate({pin: "fec615be407f2350de9c364f78a322c4ad48a2cf", parsing: parsing, registry: registry, commands: commands, rows: rows}) + "\n")
File.write(ARGV.fetch(1), JSON.pretty_generate(ActiveSupport::TimeZone::MAPPING)+"\n")
puts "WS8 slash Rails vectors: #{parsing.size} parsing, #{registry.size} registry, #{commands.size} recognition, #{rows.size} dispatch/row/callback cases"
