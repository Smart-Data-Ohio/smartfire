# Round-2 review probes. Expectations come from our actual model/job callbacks.
require_relative "ws8_vector_helpers"
%w[LIVEKIT_API_KEY LIVEKIT_API_SECRET LIVEKIT_INTERNAL_URL].each { |key| ENV.delete(key) }
TABLES.reject! { |table| table.start_with?("message_search_index") }

def reset_fixtures
  C.disable_referential_integrity do
    TABLES.each { |table| C.execute("DELETE FROM #{C.quote_table_name(table)}") }
    C.execute("DELETE FROM message_search_index")
    C.execute("DELETE FROM sqlite_sequence")
  end
  ActiveRecord::FixtureSet.reset_cache
  load File.join(__dir__, "load_fixtures.rb")
end

def deletion_fixture
  user = User.find(ActiveRecord::FixtureSet.identify("david"))
  room = Rooms::Closed.create_for({name: "Reviewer sync", creator: user}, users: [user])
  grant = HuddleGrant.create!(room: room, user: user, membership: room.memberships.find_by!(user: user), session: user.sessions.first!, identity: "review-sync-grant", room_name: "review-sync-room")
  cleanup = HuddleCleanup.create!(operation: :remove_participant, huddle_grant: grant, room_name: grant.room_name, identity: grant.identity)
  scheduled = ScheduledMessage.create!(room: room, user: user, markdown_source: "Pending", send_at: 1.hour.from_now)
  scheduled.drop!(reason: "fixture")
  [room, grant, cleanup, scheduled]
end

result = {}
%w[synchronous asynchronous].each do |path|
  reset_fixtures
  before = dump
  room, grant, cleanup, scheduled = deletion_fixture
  fixture = {now: Time.current.to_fs(:db), room_id: room.id, setup_sql: delta(before)}
  if path == "synchronous"
    room.destroy!
  else
    room.begin_destroy!
    Room::DestroyJob.perform_now(room.id)
  end
  fixture[:checks] = [check("SELECT id,revoked_at FROM huddle_grants WHERE id=#{grant.id}"), check("SELECT huddle_grant_id FROM huddle_cleanups WHERE id=#{cleanup.id}"), check("SELECT event_type FROM activity_items WHERE source_type='ScheduledMessage' AND source_id=#{scheduled.id}"), check("SELECT id FROM scheduled_messages WHERE id=#{scheduled.id}"), check("SELECT id FROM rooms WHERE id=#{room.id}")]
  result[path] = fixture
end

reset_fixtures
before = dump
room, grant, cleanup, scheduled = deletion_fixture
user = room.creator
thread = ChannelThread.create!(room: room, creator: user, name: "Resume")
501.times do |i|
  room.messages.create!(id: 2_000_000_001 + i, creator: user, body: "<p>review</p>", client_message_id: "review-#{i + 1}")
end
fixture = {now: Time.current.to_fs(:db), room_id: room.id, stop_id: 2_000_000_501, setup_sql: delta(before)}
# The general row delta does not represent an FTS virtual table reliably. Replay its actual rows.
fixture[:setup_index_sql] = C.select_rows("SELECT rowid,body FROM message_search_index WHERE rowid IN (SELECT id FROM messages WHERE room_id=#{room.id})").map { |id, body| "INSERT OR REPLACE INTO message_search_index(rowid,body) VALUES (#{C.quote(id)},#{C.quote(body)});" }.join("\n")
queries = %w[huddle_grants scheduled_messages messages channel_threads].map { |table| "SELECT COUNT(*) FROM #{table} WHERE room_id=#{room.id}" }
queries += ["SELECT COUNT(*) FROM message_search_index WHERE rowid>=2000000001", "SELECT huddle_grant_id FROM huddle_cleanups WHERE id=#{cleanup.id}", "SELECT id FROM rooms WHERE id=#{room.id}"]
room.begin_destroy!
C.execute("CREATE TRIGGER reviewer_stop BEFORE DELETE ON messages WHEN OLD.id=2000000501 BEGIN SELECT RAISE(ABORT,'interruption'); END")
begin
  Room::DestroyJob.perform_now(room.id)
  raise "missing injected interruption"
rescue ActiveRecord::StatementInvalid => error
  raise unless error.message.include?("interruption")
ensure
  C.execute("DROP TRIGGER reviewer_stop")
end
fixture[:progress] = queries.map { |sql| check(sql) }
Room::DestroyJob.perform_now(room.id)
fixture[:finished] = queries.map { |sql| check(sql) }
result[:room_resume] = fixture

reset_fixtures
before = dump
user = User.find(ActiveRecord::FixtureSet.identify("david"))
room = Room.find(ActiveRecord::FixtureSet.identify("designers"))
1001.times do |i|
  grant = HuddleGrant.create!(id: 2_000_000_001 + i, room: room, user: user, session: user.sessions.first!, membership: room.memberships.find_by!(user: user), identity: "review-#{i + 1}", room_name: "review-room", revoked_at: 31.days.ago)
  HuddleCleanup.create!(id: grant.id, operation: :remove_participant, room_name: grant.room_name, identity: grant.identity, huddle_grant: grant)
end
fixture = {now: Time.current.to_fs(:db), setup_sql: delta(before), stop_id: 2_000_001_001}
queries = ["SELECT COUNT(*) FROM huddle_grants WHERE id>=2000000001", "SELECT COUNT(*) FROM huddle_cleanups WHERE id>=2000000001 AND huddle_grant_id IS NULL"]
C.execute("CREATE TRIGGER reviewer_stop BEFORE DELETE ON huddle_grants WHEN OLD.id=2000001001 BEGIN SELECT RAISE(ABORT,'interruption'); END")
begin
  Retention::PruneJob.perform_now
  raise "missing injected interruption"
rescue ActiveRecord::StatementInvalid => error
  raise unless error.message.include?("interruption")
ensure
  C.execute("DROP TRIGGER reviewer_stop")
end
fixture[:progress] = queries.map { |sql| check(sql) }
Retention::PruneJob.perform_now
fixture[:finished] = queries.map { |sql| check(sql) }
result[:retention_resume] = fixture

reset_fixtures
before = dump
user = User.find(ActiveRecord::FixtureSet.identify("david"))
recipient = User.find(ActiveRecord::FixtureSet.identify("jason"))
room = Rooms::Closed.create_for({name: "Bookkeeping", creator: user}, users: [user, recipient])
source = room.messages.create!(creator: user, body: "<p>source</p>")
thread = ChannelThread.create!(room: room, creator: user, parent_message: source, name: "Bookkeeping thread")
thread.memberships.create!(user: recipient)
fixture = {now: Time.current.to_fs(:db), room_id: room.id, thread_id: thread.id, creator_id: user.id, source_id: source.id, setup_sql: delta(before)}
body = "<p>review /rooms/#{room.id}/@#{source.id}</p>"
fixture[:body] = body
events = []
server = ActionCable.server
original_broadcast = server.method(:broadcast)
server.define_singleton_method(:broadcast) do |stream, payload, **|
  if stream == UnreadThreadsChannel.stream_name_for(recipient.id)
    events << {kind: "cable", stream: stream, payload: payload}
  elsif payload.is_a?(String)
    frame = Nokogiri::HTML5.fragment(payload).at_css("turbo-stream[target='#{ActionView::RecordIdentifier.dom_id(source, :thread_indicator)}']")
    if frame
      label = frame.at_css(".message__thread-indicator span")&.text
      raise "missing real indicator reply count" unless label&.match?(/\A\d+ repl(?:y|ies)\z/)
      events << {kind: "indicator", parent_message_id: source.id, count: label.to_i}
    end
  end
end
subscription = ActiveSupport::Notifications.subscribe("enqueue.active_job") do |*args|
  job = args.last[:job]
  if [Room::PushMessageJob, ChannelThread::PushMessageJob].include?(job.class)
    conversation, message = job.arguments
    events << {kind: "push", class: job.class.name, conversation_id: conversation.id, message_id: message.id}
  end
end
begin
  root = room.messages.create!(creator: user, body: body, client_message_id: "review-root")
  reply = room.messages.create!(creator: user, thread: thread, body: body, client_message_id: "review-thread")
ensure
  server.define_singleton_method(:broadcast, original_broadcast)
  ActiveSupport::Notifications.unsubscribe(subscription)
end
fixture[:events] = events
fixture[:root_id] = root.id
fixture[:reply_id] = reply.id
queries = ["SELECT id,client_message_id,room_id,thread_id,updated_at FROM messages WHERE id IN (#{root.id},#{reply.id},#{source.id}) ORDER BY id", "SELECT rowid,body FROM message_search_index WHERE rowid IN (#{root.id},#{reply.id}) ORDER BY rowid", "SELECT user_id,unread_at,last_read_message_id,updated_at FROM memberships WHERE room_id=#{room.id} ORDER BY user_id", "SELECT user_id,unread_at,updated_at FROM thread_memberships WHERE thread_id=#{thread.id} ORDER BY user_id", "SELECT message_id,referenced_message_id FROM message_references WHERE message_id IN (#{root.id},#{reply.id}) ORDER BY message_id", "SELECT messages_count,updated_at FROM channel_threads WHERE id=#{thread.id}"]
fixture[:checks] = queries.map { |sql| check(sql) }
result[:bookkeeping] = fixture

File.write(ARGV.fetch(0), JSON.pretty_generate(result) + "\n")
puts "WS8 round-2 vectors: 2 destruction paths, 501-message resume, 1001-grant resume, 6 bookkeeping checks, #{events.size} ordered callback events"
