# Actual shared collection-cache states and keys; render twice through the same Rails cache.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.cache = ActiveSupport::Cache::MemoryStore.new
user = User.find_by!(email_address: "david@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
message = room.root_messages.create!(creator: user, markdown_source: "Collection", client_message_id: "collection-target")
renderer = ApplicationController.renderer.new(http_host: "campfire.test", https: false)
helper = ApplicationController.helpers
states = []
capture = ->(name, &operation) do
  travel_to Time.utc(2026, 3, 2, 16) + states.size
  operation&.call
  message.reload
  Message.preload_rendering_details([message])
  key = helper.message_with_pr_cards_cache_key(message)
  html = 2.times.map { renderer.render(partial: "messages/message", collection: [message], cached: ->(record) { helper.message_with_pr_cards_cache_key(record) }) }
  raise "cache bytes differ" unless html.uniq.one?
  raise "session value in collection" if html.first.include?("authenticity_token") || html.first.match?(/nonce="[^"]+/)
  states << { name:, time: Time.current.iso8601, key: ActiveSupport::Cache.expand_cache_key(key), html: html.first }
end
capture.call("initial")
pin = nil
capture.call("pinned") { pin = MessagePin.create!(message:, room:, pinner: user) }
capture.call("unpinned") { pin.destroy! }
thread = nil
capture.call("thread") { thread = ChannelThread.create!(room:, creator: user, parent_message: message, name: "Collection thread") }
capture.call("thread_reply") { thread.messages.create!(room:, creator: user, markdown_source: "Child", client_message_id: "collection-child") }
capture.call("streaming") { message.update_columns(streaming: true) }
capture.call("final") { message.update_columns(streaming: false) }
capture.call("edited") { message.update!(markdown_source: "## Edited", edited_at: Time.current) }
capture.call("drive") { message.drive_attachments.create!(file_id: "abcdefghij") }
capture.call("reaction") { Boost.create!(message:, booster: user, content: "👍") }
quote_names = [
  [[user.name, nil]],
  [[user.name, nil], ["Other", room.name]],
  [[user.name, nil], [user.name, room.name]]
].map do |names|
  begin
    { names:, digest: Digest::SHA256.hexdigest(names.sort.inspect) }
  rescue ArgumentError => error
    { names:, error: error.message }
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], message_id: message.id, thread_id: thread.id, states:, quote_names:) + "\n")
puts "WS8bm collection oracle: #{states.size} real Rails states; keys and cache-hit bytes; 0 session-bound values"
