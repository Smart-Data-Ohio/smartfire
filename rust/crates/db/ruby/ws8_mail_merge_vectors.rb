require "json"
load "/tools/load_fixtures.rb"
ActiveJob::Base.queue_adapter = :test
Rails.cache = ActiveSupport::Cache::MemoryStore.new
ENV["INBOUND_EMAIL_DOMAIN"] = "mail.test"
user = User.find(ActiveRecord::FixtureSet.identify("david"))
room = Room.find(ActiveRecord::FixtureSet.identify("designers"))
validation = [
  {name: "normal", unit: "# hi", count: 1},
  {name: "blank", unit: " \n\t", count: 1},
  {name: "streaming_blank", unit: " \n\t", count: 1, streaming: true},
  {name: "drive_blank", unit: " \n\t", count: 1, drive_file_ids: ["ws8-mail-file"]},
  {name: "boundary", unit: "é", count: 50_000},
  {name: "overlong", unit: "é", count: 50_001}
].map do |sample|
  message = room.messages.new(creator: user, markdown_source: sample[:unit] * sample[:count], streaming: sample[:streaming] || false, action: true, embeds_suppressed: true, reply_notify_author: false)
  Array(sample[:drive_file_ids]).each { |file_id| message.drive_attachments.build(file_id:) }
  message.valid?
  sample.merge(errors: message.errors.map { |error| [error.attribute, error.message] }, metadata: message.attributes.slice("action", "embeds_suppressed", "reply_notify_author"))
end
direct = Rooms::Direct.create_for({creator: user}, users: [user, User.find(ActiveRecord::FixtureSet.identify("jason"))])
direct.update_column(:name, "é" * 101)
begin
  direct.regenerate_inbound_email_token!
rescue ActiveRecord::RecordInvalid => error
  token_errors = error.record.errors.map { |entry| [entry.attribute, entry.message] }
end
raise "invalid direct name should reject token rotation" unless token_errors

room.reload.update!(inbound_email_token: "ws8-mail-merge-token")
raw = "Message-ID: <ws8-mail-merge@example.com>\r\nFrom: outside@example.com\r\nTo: room-ws8-mail-merge-token@mail.test\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n# hi\r\n"
inbound = ActionMailbox::InboundEmail.create_and_extract_message_id!(raw)
# Capture the real broadcasts without contacting an external pubsub server.
server = ActionCable.server
original = server.method(:broadcast)
server.define_singleton_method(:broadcast) { |*args, **kwargs| }
begin
  RoomMailbox.new(inbound).process
ensure
  server.define_singleton_method(:broadcast, original)
end
message = room.messages.order(:id).last
mail = {raw: raw, source: message.markdown_source, body: message.body.body.to_html,
        plain: Message::Markdown.plain_text(message.body.body), creator_name: message.creator.name}
File.write(ARGV.fetch(0), JSON.pretty_generate({validation: validation, token_errors: token_errors, mail: mail}) + "\n")
puts "WS8 mail merge vectors: #{validation.size} Rails validations, 1 token validation, 1 real RoomMailbox post"
