# Rails oracle at the current reference pin, run over a private copy of the default parity seed.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16) do
  bot = User.find(394959859)
  room = Room.find(486777696)
  Current.user = bot
  message = room.messages.create!(creator: bot, body: "Reply token message")
  token = bot.reply_token_for(room)
  client = ActionDispatch::Integration::Session.new(Rails.application)
  client.host! "campfire.test"
  client.https!
  base = Rails.application.routes.url_helpers.room_bot_messages_path(room, token)
  cases = [["get", base], ["put", "#{base}/#{message.id}"], ["delete", "#{base}/#{message.id}"],
           ["post", "#{base}/#{message.id}/boosts"], ["delete", "#{base}/#{message.id}/boosts/0"]]
  denied = cases.map do |method, path|
    client.public_send(method, path, params: "Forbidden", headers: { "CONTENT_TYPE" => "text/plain" })
    { method: method, status: client.response.status }
  end
  missing_reply_messages = %w[put delete].map do |method|
    client.public_send(method, "#{base}/0", params: "Missing", headers: { "CONTENT_TYPE" => "text/plain" })
    { method: method, status: client.response.status }
  end
  plain_key = "#{bot.id}-BenderToken1"
  note = room.messages.create!(creator: bot, body: "System note", system_note: true)
  note_path = Rails.application.routes.url_helpers.room_bot_message_path(room, plain_key, note)
  system_note = %w[put delete].map do |method|
    client.public_send(method, note_path, params: "Changed", headers: { "CONTENT_TYPE" => "text/plain" })
    { method: method, status: client.response.status }
  end
  count = room.root_messages.count
  thread = room.channel_threads.create!(creator: bot, name: "Bot pagination")
  reply = thread.post_message!(creator: bot, attributes: { body: "Thread reply" })
  client.get Rails.application.routes.url_helpers.room_bot_messages_path(room, plain_key)
  page = { status: client.response.status, total: client.response.headers["X-Total-Count"].to_i,
           expected_total: count, excludes_reply: client.response.parsed_body.none? { |m| m["id"] == reply.id } }
  agent = bot.agent
  secret = "ws11-test-credential"
  digest = AgentCredential.digest(secret)
  credential = AgentCredential.create!(agent: agent, created_by: User.find(127326141), name: "WS11",
    token_digest: digest, token_last_four: digest[0, 4])
  headers = { "Authorization" => ["Bearer", secret].join(" ") }
  auth = []
  client.get "/rooms/#{room.id}", headers: headers
  auth << { name: "valid credential human endpoint", status: client.response.status }
  credential.update!(revoked_at: Time.current)
  client.get "/rooms/#{room.id}", headers: headers
  auth << { name: "revoked", status: client.response.status }
  credential.update!(revoked_at: nil, expires_at: Time.current)
  client.get "/rooms/#{room.id}", headers: headers
  auth << { name: "expired at exact boundary", status: client.response.status }
  key_path = Rails.application.routes.url_helpers.room_bot_messages_path(room, plain_key)
  grants = []
  client.get key_path
  grants << { name: "legacy", status: client.response.status }
  grant = agent.agent_grants.create!(capability: "read_messages", granted_by: User.find(127326141), revoked_at: Time.current)
  client.get key_path
  grants << { name: "revoked only", status: client.response.status, body: client.response.parsed_body }
  grant.update!(revoked_at: nil)
  client.get key_path
  grants << { name: "workspace-wide", status: client.response.status }
  client.post key_path, params: "Missing posting grant", headers: { "CONTENT_TYPE" => "text/plain" }
  grants << { name: "missing post", status: client.response.status, body: client.response.parsed_body }
  grant.update!(room: Room.find(340026324))
  client.get key_path
  grants << { name: "wrong room", status: client.response.status }
  grant.update!(room: room)
  client.get key_path
  grants << { name: "room scoped", status: client.response.status }
  grant.revoke!
  client.get key_path
  grants << { name: "revoked next request", status: client.response.status }
  puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], reply_denials: denied, missing_reply_messages: missing_reply_messages, system_note_denials: system_note, root_page: page, credential_digest: digest, credential_auth: auth, grants: grants)
end
