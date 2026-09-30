# Boots the reference app on a database campfire_db wrote (`export_database_for_rails` in
# src/tests/fixtures_test.rs) and reads, edits, searches and deletes through Active Record, the
# way Rails would after a rollback. Run by reference-tools/db/differential.sh.
ActiveJob::Base.queue_adapter = :test
def check(what) = (yield or raise "rollback: #{what}")

message = Message.find_by!(client_message_id: "rust-1")
check("message body") { message.plain_text_body == "Written by Rust hovercraft" }
check("message room and creator") { message.room.name == "Designers" && message.creator.name == "David" }
check("boost") { message.boosts.sole.then { _1.content == "🦀" && _1.booster.name == "Jason" } }
check("search finds the Rust message") { Message.search("hovercraft").include?(message) }

rusty = User.find_by!(email_address: "rusty@example.com")
check("password") { rusty.authenticate("secret123456") }
check("sessions, device and two-factor columns") do
  rusty.sessions.order(:id).map { [ _1.ip_address, _1.device_id, _1.two_factor_verified? ] } ==
    [ [ "8.8.8.8", nil, false ], [ "9.9.9.9", "rust-device", true ] ]
end
check("search record") { rusty.searches.pluck(:query) == [ "hovercraft" ] }
room = Room.find_by!(name: "Rust Room")
check("closed room") { room.is_a?(Rooms::Closed) && room.users.pluck(:name).sort == %w[ David Rusty ] }
david, kevin, jason = %w[ David Kevin Jason ].map { User.find_by!(name: _1) }

check("room types") do
  { "Rust Voice" => Rooms::Voice, "Rust Stage" => Rooms::Stage, "Rust Board" => Rooms::Board }.all? do |name, type|
    Room.find_by!(name: name).then { _1.instance_of?(type) && _1.users.sort == [ david, rusty ].sort }
  end
end
stage = Room.find_by!(name: "Rust Stage")
check("stage roles") { stage.memberships.find_by!(user: rusty).host? && stage.memberships.find_by!(user: david).listener? }
direct = Rooms::Direct.find_for([ jason, rusty, kevin ])
check("direct room found by its member key") do
  direct&.direct_member_key == Rooms::Direct.member_key_for([ rusty.id, kevin.id, jason.id ]) &&
    direct.users.sort == [ jason, kevin, rusty ].sort &&
    Rooms::Direct.find_or_create_for([ kevin, jason, rusty ]) == direct
end
check("muted involvement") { room.memberships.find_by!(user: rusty).involved_in_muted? }
check("unread for the disconnected member") { room.memberships.find_by!(user: david).unread? }

tied = room.root_messages.where(client_message_id: [ "rust-tie one", "rust-tie two", "rust-tie three" ]).ordered.to_a
check("messages ordered by (created_at, id)") do
  tied.map(&:plain_text_body) == [ "tie one", "tie two", "tie three" ] && tied.map(&:created_at).uniq.size == 1
end
check("pagination breaks the tie by id") do
  room.root_messages.page_before(tied.last).last(2) == tied.first(2) &&
    room.root_messages.page_after(tied.first).first(2) == tied.last(2)
end
note = Message.find_by!(client_message_id: "rust-note")
check("system note") { note.system_note? && Message.search("renamed").exclude?(note) }
reply = Message.find_by!(client_message_id: "rust-reply")
check("thread reply") do
  reply.thread_message? && reply.thread.messages == [ reply ] && room.root_messages.exclude?(reply) && room.messages.include?(reply)
end

check("thread counter") { reply.thread.messages_count == 1 }
check("thread membership") do
  reply.thread.memberships.order(:id).map { [ _1.user.name, _1.involvement ] } == [ [ "Rusty", "mentions" ], [ "David", "everything" ] ]
end
board = Room.find_by!(name: "Rust Board")
check("board root holds only the system note") do
  board.root_messages.pluck(:client_message_id) == [ "rust-board-note" ] &&
    Message.find_by!(client_message_id: "rust-board-post").thread.room == board
end
check("board post tags") { board.channel_threads.sole.tags.pluck(:name) == %w[ rust-tag ui ] && board.channel_threads.sole.work_status == "planned" }
stream = Message.find_by!(client_message_id: "rust-stream")
check("stream activity clock") do
  stream.streaming? && stream.streaming_updated_at == stream.created_at + 9.minutes &&
    Message.overdue_streams(now: stream.created_at + 11.minutes).exclude?(stream) &&
    Message.overdue_streams(now: stream.streaming_updated_at + 11.minutes).include?(stream)
end

orphan = Message.find_by!(client_message_id: "rust-reply-to-doomed")
check("reply tombstone") do
  Message.where(client_message_id: "rust-doomed").none? && orphan.reply? && orphan.reply_to_message.nil? &&
    orphan.reply_target_deleted_at.present?
end

edited = Message.find_by!(client_message_id: "rust-edited")
check("edit markers") do
  edited.edited_at.present? && edited.markdown_source == "after" && edited.embeds_suppressed? &&
    edited.drive_attachments.pluck(:file_id) == [ "2BcdEfgHiJkLmNoPqRsTu" ]
end
quiet = Message.find_by!(client_message_id: "rust-quiet-reply")
check("quiet reply") { quiet.reply_to_message == edited && !quiet.reply_notify_author? }
forwards = Message.where(forwarded_from_message: edited).order(:id)
check("forwards") do
  forwards.map { _1.room.name } == [ "All Talk", "Designers" ] &&
    forwards.all? { _1.forwarded? && _1.forwarded_markdown? && !_1.markdown? && _1.mentionees.to_a == [ jason ] } &&
    forwards.first.plain_text_body.start_with?("@[Jason] look\n\n")
end

poll = Message.find_by!(client_message_id: "rust-poll").poll
check("closed poll") do
  poll.closed? && poll.multiple? && poll.poll_options.map(&:label) == %w[ Tacos Pizza ] &&
    poll.results_payload(viewer: david)[:options].map { [ _1[:votes], _1[:voters], _1[:voted] ] } ==
      [ [ 2, %w[ David Jason ], true ], [ 1, %w[ David ], true ] ]
end
later = Message.find_by!(client_message_id: "rust-poll-later").poll
check("open poll") { later.open? && later.anonymous? && Poll.open.include?(later) && Poll.open.exclude?(poll) }

designers = Room.find_by!(name: "Designers")
pinned = Message.find_by!(client_message_id: "0001")
check("pins") do
  MessagePin.pinned?(pinned) && !MessagePin.pinned?(Message.find_by!(client_message_id: "0002")) &&
    designers.message_pins.sole.pinner == david && designers.pins_changed_at.present?
end
check("pin notes") do
  notes = designers.root_messages.where(system_note: true).order(:id)
  notes.map(&:markdown_source) == [
    "pinned a message: [jump to message](/rooms/#{designers.id}/@#{pinned.id})",
    "pinned a message: [jump to message](/rooms/#{designers.id}/@#{Message.find_by!(client_message_id: "0002").id})"
  ] && notes.map(&:creator) == [ david, jason ] && notes.first.plain_text_body.include?("jump to message")
end
reminded = SavedItem.find_by!(user: david, message: pinned)
check("fired reminder") do
  reminded.reminder_fired? && reminded.in_progress? &&
    ActivityItem.find_by!(user: david, source: reminded).then { _1.event_type == "message_reminder" && _1.unread? } &&
    SavedItem.accessible_to(david).include?(reminded) && SavedItem.due_reminders(reminded.remind_at + 1.day).exclude?(reminded)
end
done = SavedItem.find_by!(user: jason)
check("done saved item") { done.done? && done.reminder_pending? && SavedItem.due_reminders(done.remind_at).include?(done) }

check("keyword normalization") { david.keyword_alerts.sole.phrase == "Deploy freeze" }
check("category and favorite") do
  category = david.room_categories.sole
  membership = david.memberships.find_by!(room: designers)
  category.name == "Team" && category.collapsed? && category.position == 1 &&
    membership.room_category == category && membership.favorite_position == 0 &&
    david.memberships.favorites.include?(membership)
end
scheduled = ScheduledMessage.find_by!(markdown_source: "Scheduled hello")
check("sent schedule") do
  scheduled.sent? && scheduled.sent_message.plain_text_body == "Scheduled hello" &&
    scheduled.sent_message.creator == david && scheduled.sent_message.reply_to_message == pinned
end
pending = ScheduledMessage.find_by!(markdown_source: "Pending hello")
check("pending schedule") { pending.pending? && pending.sendable? && ScheduledMessage.owned_by(david).include?(pending) }
check("audit snapshots and filtering") do
  entry = AuditLog.find_by!(action: "room.membership.change")
  entry.actor_label == "David <david@37signals.com>" && entry.target_type == "Room" &&
    entry.target_label == "Designers" && entry.details["token"] == "[FILTERED]" && entry.readonly?
end
group = Rooms::Direct.find_by!(name: "Rust Group")
check("group mutations") do
  group.users.pluck(:name).sort == %w[ David JZ Jason ].sort &&
    group.direct_member_key == Rooms::Direct.member_key_for(group.user_ids) &&
    group.messages.where(system_note: true).ordered.map(&:plain_text_body) ==
      [ "renamed the group to Rust Group", "added JZ to the group", "left the group" ]
end
check("quote references") { Message.find_by!(client_message_id: "rust-quote").referenced_messages.to_a == [ pinned ] }
dropped = ScheduledMessage.find_by!(markdown_source: "Dropped hello")
check("dropped schedule") do
  dropped.dropped? && dropped.drop_reason == "its thread was deleted" &&
    dropped.activity_items.sole.event_type == "scheduled_message_dropped"
end

deleted_room=Room.find_by!(name: "Rust deleted room")
old_huddle_secret=ENV["LIVEKIT_API_SECRET"]
ENV["LIVEKIT_API_SECRET"]="fixture-secret"
cleanup_room_name=Huddle.room_name(deleted_room.id)
ENV["LIVEKIT_API_SECRET"]=old_huddle_secret
check("room deletion and cleanup claim") do
  deleted_room.deleted? && deleted_room.memberships.empty? && deleted_room.destroy_enqueued_at.present? &&
    HuddleCleanup.where(operation: "delete_room",room_name: cleanup_room_name).any? { |r| r.valid? && r.enqueued_at.present? && r.next_attempt_at == r.enqueued_at + 1.minute }
end

bot = User.find_by!(name: "Rust Bot")
bot_key = File.read("/out/rust_export.sqlite3.bot_key")
tampered = bot_key.sub(/.\z/) { _1 == "a" ? "b" : "a" }
check("bot key authenticates by digest") do
  bot.bot_token.nil? && User.authenticate_bot(bot_key) == bot && User.authenticate_bot(tampered).nil?
end

# Every row Rust inserted or changed: what differs from the reference's own fixtures, which
# the Rust export loaded at the same instant (fixtures_match_ruby_row_for_row proves the loads
# identical). Password digests are left out: the export hashes at a lower cost, with its own salt.
connection = ActiveRecord::Base.connection
connection.execute("ATTACH DATABASE '/out/fixtures_ruby.sqlite3' AS fixtures")
Rails.application.eager_load!
models = ActiveRecord::Base.descendants.reject(&:abstract_class?).select { _1 == _1.base_class }.index_by(&:table_name)
tables = connection.select_values(<<~SQL)
  SELECT name FROM main.sqlite_master WHERE type = 'table' AND sql NOT LIKE 'CREATE VIRTUAL%'
    AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'message_search_index%'
    AND name NOT IN ('ar_internal_metadata', 'schema_migrations')
SQL
rust_written = {}
tables.each do |table|
  columns = (connection.columns(table).map(&:name) - [ "password_digest" ]).map { connection.quote_column_name(_1) }.join(", ")
  changed = connection.select_values("SELECT id FROM (SELECT #{columns} FROM main.#{table} EXCEPT SELECT #{columns} FROM fixtures.#{table})")
  rust_written[table] = changed if changed.any?
end
connection.execute("DETACH DATABASE fixtures")
unmodeled = rust_written.keys - models.keys
check("every table Rust wrote has a model: #{unmodeled}") { unmodeled.empty? }
records = rust_written.flat_map { |table, ids| models.fetch(table).where(id: ids).to_a }
invalid = records.reject(&:valid?).map { [ _1.class.name, _1.id, _1.errors.full_messages ] }
check("Rails validates every row Rust wrote: #{invalid}") do
  records.size == rust_written.values.sum(&:size) && %w[ messages boosts searches channel_threads thread_memberships thread_tags memberships rooms users sessions ].all? { rust_written.key?(_1) } &&
    invalid.empty?
end
puts "validated #{records.size} rows Rust wrote: #{rust_written.transform_values(&:size).sort.to_h}"

check("account settings") { Account.first.settings.restrict_room_creation_to_administrators == true }

Current.user = rusty
message.update!(body: "Edited by Rails zeppelin")
check("edit is searchable") { Message.search("zeppelin").include?(message) && Message.search("hovercraft").exclude?(message) }
message.destroy!
check("delete") { Message.search("zeppelin").none? && Boost.where(message_id: message.id).none? }
puts "rollback ok"
