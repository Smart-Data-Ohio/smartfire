# Rails oracle at d7c7de92, run over a private copy of the default parity seed.
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
  puts JSON.pretty_generate(reference_pin: "d7c7de92", reply_denials: denied, system_note_denials: system_note, root_page: page)
end
