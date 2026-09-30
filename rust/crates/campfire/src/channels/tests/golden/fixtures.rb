# Fixtures for recording reference.json (see ../golden.rs): creates the users, rooms, sessions
# and message the script needs, and prints the rows the Rust replay loads, the session cookies
# and the signed stream names. Run with
# `parity/bin/reference runner --port N crates/campfire/src/channels/tests/golden/fixtures.rb`.
Account.first || Account.create!(name: "Campfire")

def golden_user(name, email)
  User.find_by(email_address: email) || User.create!(name: name, email_address: email, password: "secret123456")
end

a = golden_user("Channel Tester", "channels-a@example.com")
b = golden_user("Channel Peer", "channels-b@example.com")
c = golden_user("Channel Outsider", "channels-c@example.com")
room = Rooms::Open.find_by(name: "Channels") || Rooms::Open.create_for({ name: "Channels", creator: a }, users: [ a, b ])
closed = Rooms::Closed.find_by(name: "Channels Private") || Rooms::Closed.create_for({ name: "Channels Private", creator: c }, users: [ c ])
message = room.messages.find_by(client_message_id: "channels-golden-1") ||
  room.messages.create!(body: "Hello", creator: b, client_message_id: "channels-golden-1")
thread = room.channel_threads.create!(name: "Golden thread", creator: a)
bot = User.create!(name: "Golden Bot", role: :bot, bot_token_digest: User.digest_bot_token("test-only-golden-token"))
room.memberships.grant_to(bot)

cookie = ->(user, verified = true) do
  session = user.sessions.start!(user_agent: "golden", ip_address: "8.8.8.8")
  # Our app rejects cable connections from people whose session hasn't completed the second
  # factor (ApplicationCable::Connection, TwoFactorEnforcement).
  session.mark_two_factor_verified! if verified
  request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "127.0.0.1", "rack.input" => StringIO.new))
  request.cookie_jar.signed[:session_token] = session.token
  "session_token=#{CGI.escape(request.cookie_jar[:session_token])}"
end
cookies = { "A" => cookie.(a), "B" => cookie.(b), "BOT" => cookie.(bot, false), "PENDING" => cookie.(a, false) }

rows = ->(sql) { ActiveRecord::Base.connection.select_all(sql).to_a }
user_ids = [ a, b, c, bot ].map(&:id).join(",")

puts JSON.generate(
  cookies: cookies,
  tokens: {
    "A_ID" => a.id.to_s,
    "B_ID" => b.id.to_s,
    "ROOM_ID" => room.id.to_s,
    "CLOSED_ID" => closed.id.to_s,
    "MESSAGE_ID" => message.id.to_s,
    "THREAD_ID" => thread.id.to_s,
    "ROOMS_SIGNED" => Turbo::StreamsChannel.signed_stream_name(:rooms),
    "A_ROOMS_SIGNED" => Turbo::StreamsChannel.signed_stream_name([ a, :rooms ]),
    "ROOM_MESSAGES_SIGNED" => Turbo::StreamsChannel.signed_stream_name([ room, :messages ]),
    "ROOM_THREADS_SIGNED" => Turbo::StreamsChannel.signed_stream_name([ room, :threads ]),
    "THREAD_MESSAGES_SIGNED" => Turbo::StreamsChannel.signed_stream_name([ thread, :messages ]),
    "A_STATUS_SIGNED" => Turbo::StreamsChannel.signed_stream_name([ a, :status ]),
    "A_OOO_SIGNED" => Turbo::StreamsChannel.signed_stream_name([ a, :ooo_notice ]),
    "CLOSED_MESSAGES_SIGNED" => Turbo::StreamsChannel.signed_stream_name([ closed, :messages ])
  },
  rows: {
    "users" => rows.("SELECT * FROM users"),
    "rooms" => rows.("SELECT * FROM rooms"),
    "memberships" => rows.("SELECT * FROM memberships"),
    "sessions" => rows.("SELECT * FROM sessions WHERE user_id IN (#{user_ids})"),
    "messages" => rows.("SELECT * FROM messages WHERE id = #{message.id}"),
    "channel_threads" => rows.("SELECT * FROM channel_threads WHERE id = #{thread.id}")
  }
)
