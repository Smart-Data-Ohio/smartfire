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

texts = ["", "sometime-ish", "in zero minutes", "in 0 minutes", "in 1 min", "in 20 minutes review the deploy", "in 3 hrs", "in 2 days", "in 1 week", "in 2 weeks", "tomorrow", "tomorrow at 2:30am", "tomorrow 2:15am", "tomorrow 9:30pm", "today at 3pm", "today at 1am", "at 15:00", "at 0am", "at 24:00", "at 12:60", "at 12pm", "at 12am", "friday", "friday 5pm", "next friday", "next sunday 2:30am", "sunday 1:30am", "wednesday 5pm", "mondayland", "tomorrowish", "2026-03-08 02:30", "2026-11-01 01:30", "2026-02-30 15:00", "2026-10-01T15:00:05", "2026-10-01", "5pm", "17:30", "Oct 5", "review the deploy friday", "Launch party friday 5pm", "Friday", "Party", "  tomorrow 9am  hello\nworld  ", "next friday 9am trailing"]
clocks = ["2026-09-23 12:00:00", "2026-03-07 17:00:00", "2026-03-08 06:59:00", "2026-11-01 04:30:00", "2026-11-01 06:30:00", "2026-10-03 15:00:00", "2011-12-29 12:00:00", "1985-12-31 12:00:00"]
zones = ["UTC", "America/New_York", "Eastern Time (US & Canada)", "Asia/Kathmandu", "Pacific/Apia", "Australia/Lord_Howe", "Invalid/Zone"]
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
inputs = ["/poll", "/poll ignored", "/wat", "//poll", "/huddle", "/event", "/event Launch party friday 5pm", "/event Launch party", "/event Retro 2026-09-01 15:00", "/shrug", "/shrug ship it", "/me waves", "/me", "/remind", "/remind sometime review", "/remind in 20 minutes", "/remind in 20 minutes review deploy", "/remind 2026-09-01 15:00 old", "/status 🚂 On a train", "/status", "/status abcdefghi Too long", "/status 🚂 " + "x"*101, "/dnd", "/dnd off", "/dnd ON", "/dnd 2h", "/dnd 3d", "/dnd 0m", "/dnd until 5pm", "/dnd until at 5pm", "/dnd eventually", "/ooo 2h Back soon", "/ooo 1 week", "/ooo friday 5pm Wrapping up", "/ooo tomorrow Back soon", "/ooo today", "/ooo friday", "/ooo next wednesday", "/ooo 2026-10-05", "/ooo oct 8 Back soon", "/ooo sep 1", "/ooo feb 30", "/ooo feb 29", "/ooo 3d", "/ooo 0m", "/ooo tomorrow " + "x"*141, "/ooo off", "/play rimshot", "/play", "/SHRUG hi\nthere"]
inputs.product(["root", "thread", "locked", "board"]).each do |text, place|
  begin
    copy_db(ActiveRecord::Base.connection.raw_connection, baseline)
    ActiveRecord::Base.connection.clear_query_cache
    travel_to Time.utc(2026,9,23,12)
    user = User.find(ActiveRecord::FixtureSet.identify("david")); user.update!(time_zone: "America/New_York")
    room = Room.find(ActiveRecord::FixtureSet.identify("watercooler"))
    if place == "board"
      room.update!(type: "Rooms::Board"); room = Room.find(room.id)
    end
    thread = ChannelThread.create!(room: room, creator: user, name: "Slash test") if %w[thread locked].include?(place)
    thread.update!(locked_at: Time.current) if place == "locked"
    $calls = []; ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    result = SlashCommands::Dispatcher.dispatch(user: user, room: room, thread: thread, text: text)
    message = Message.find_by(id: result.payload[:message_id])
    user.reload
    selected = %w[custom_status_emoji custom_status_text custom_status_expires_at dnd_enabled dnd_until ooo_until ooo_note ooo_broadcast updated_at]
    state = selected.to_h { |key| v=user.public_send(key); [key, v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? stamp(v) : v] }
    message_state = message && message.attributes.slice("action","system_note","streaming","markdown_source","created_at","updated_at").transform_values { |v| v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? stamp(v) : v }
    saved = message && SavedItem.find_by(user: user,message: message)
    rows << {text: text, place: place, result: {kind: result.kind, message: result.message, notice: result.notice, url: result.url}, user: state, message: message_state, saved: saved && {remind_at: stamp(saved.remind_at), status: saved.status, reminded_at: stamp(saved.reminded_at)}, jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j| j[:job].name }.sort, broadcasts: $calls}

  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate({pin: "fec615be407f2350de9c364f78a322c4ad48a2cf", parsing: parsing, registry: registry, commands: commands, rows: rows}) + "\n")
File.write(ARGV.fetch(1), JSON.pretty_generate(ActiveSupport::TimeZone::MAPPING)+"\n")
puts "WS8 slash Rails vectors: #{parsing.size} parsing, #{registry.size} registry, #{commands.size} recognition, #{rows.size} dispatch/row/callback cases"
