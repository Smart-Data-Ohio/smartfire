# Which timestamps each Message save path advances in the reference app, for
# `save_touches_match_ruby` (src/tests/save_touches_test.rs). Every case gets its own room and
# message at T0, runs one action at T0 + 60s, and records how far past T0 the message's
# updated_at and streaming_updated_at and the room's updated_at ended up (nil when unset or
# destroyed).
#
#   RAILS_ENV=test bin/rails runner path/to/save_touches.rb /out/save_touches_ruby.json
require "active_support/testing/time_helpers"
ActiveJob::Base.queue_adapter = :test
extend ActiveSupport::Testing::TimeHelpers

def fx(model, label) = model.find(ActiveRecord::FixtureSet.identify(label))
david, jason = %i[david jason].map { fx(User, _1) }
Current.user = david

def blob(name) = ActiveStorage::Blob.create_and_upload!(io: StringIO.new(name), filename: "#{name}.txt", content_type: "text/plain")

ACTIONS = {
  "attach_nil"      => ->(m, _) { m.update!(attachment: nil) },
  "attach_blob"     => ->(m, _) { m.update!(attachment: blob("new")) },
  "body_same"       => ->(m, _) { m.update!(body: m.body.body.to_html) },
  "body_new"        => ->(m, _) { m.update!(body: "rewritten") },
  "touch"           => ->(m, _) { m.touch },
  "boost_create"    => ->(m, u) { m.boosts.create!(content: "hi", booster: u) },
  "boost_destroy"   => ->(m, _) { m.boosts.first.destroy! },
  "destroy"         => ->(m, _) { m.destroy! },
  "markdown_new"    => ->(m, _) { m.update!(markdown_source: "rewritten") },
  "embeds_suppress" => ->(m, _) { m.update!(embeds_suppressed: true) },
  "forward_note"    => ->(m, _) { m.update!(forward_note: "note") },
  "drive_add"       => ->(m, _) { m.drive_attachments.build(file_id: "1AbcDefGhIjKlMnOpQrSt"); m.save! },
  "pin"             => ->(m, u) { MessagePin.pin!(message: m, pinner: u) },
  "unpin"           => ->(m, _) { MessagePin.find_by!(message: m).unpin! },
  "save_item"       => ->(m, u) { SavedItem.create!(user: u, message: m, remind_at: 1.hour.from_now) },
  "schedule"        => ->(m, u) { ScheduledMessage.create!(user: u, room: m.room, reply_to_message: m, markdown_source: "Later", send_at: 1.hour.from_now) }
}

t0 = Time.utc(2026, 9, 29, 12, 0, 0)
results = {}
[ false, true ].product([ false, true ], ACTIONS.keys).each do |streaming, attached, action|
  name = "#{streaming ? "streaming" : "finished"}/#{attached ? "attached" : "bare"}/#{action}"
  travel_to t0
  room = Rooms::Open.create!(name: name, creator: david)
  attributes = { body: "partial", creator: david, streaming: streaming, client_message_id: name }
  attributes[:attachment] = blob("old") if attached
  message = room.messages.create!(attributes)
  message.boosts.create!(content: "yo", booster: jason) if action == "boost_destroy"
  MessagePin.pin!(message:, pinner: jason) if action == "unpin"
  travel_to t0 + 60.seconds
  ACTIONS.fetch(action).call(Message.find(message.id), jason)
  travel_back
  message = Message.find_by(id: message.id)
  room.reload
  offset = ->(time) { time && (time - t0).round }
  results[name] = {
    "message_updated_at" => offset.(message&.updated_at),
    "streaming_updated_at" => offset.(message&.streaming_updated_at),
    "room_updated_at" => offset.(room.updated_at),
    "attached" => !!message&.attachment&.attached?
  }
end

File.write(ARGV.fetch(0), JSON.pretty_generate(results) + "\n")
puts "save touches: #{results.size} cases"
