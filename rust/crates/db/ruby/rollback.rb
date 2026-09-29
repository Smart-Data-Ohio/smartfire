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
board = Room.find_by!(name: "Rust Board")
check("board root holds only the system note") do
  board.root_messages.pluck(:client_message_id) == [ "rust-board-note" ] &&
    Message.find_by!(client_message_id: "rust-board-post").thread.room == board
end
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

bot = User.find_by!(name: "Rust Bot")
bot_key = File.read("/out/rust_export.sqlite3.bot_key")
tampered = bot_key.sub(/.\z/) { _1 == "a" ? "b" : "a" }
check("bot key authenticates by digest") do
  bot.bot_token.nil? && User.authenticate_bot(bot_key) == bot && User.authenticate_bot(tampered).nil?
end

rust_written = [
  *Message.where("client_message_id LIKE 'rust-%'"), *Room.where("name LIKE 'Rust %'"),
  *Membership.where(user: rusty), *rusty.sessions, rusty, bot, *Rooms::Direct.find_for([ jason, rusty, kevin ])
]
check("Rails validates every row Rust wrote: #{rust_written.reject(&:valid?).map { [ _1.class.name, _1.id, _1.errors.full_messages ] }}") do
  rust_written.size > 20 && rust_written.all?(&:valid?)
end

check("account settings") { Account.first.settings.restrict_room_creation_to_administrators == true }

Current.user = rusty
message.update!(body: "Edited by Rails zeppelin")
check("edit is searchable") { Message.search("zeppelin").include?(message) && Message.search("hovercraft").exclude?(message) }
message.destroy!
check("delete") { Message.search("zeppelin").none? && Boost.where(message_id: message.id).none? }
puts "rollback ok"
