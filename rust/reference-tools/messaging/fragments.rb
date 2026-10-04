# Whole message partials from real default-seed records, rendered through Rails' fragment cache.
require "json"
require "active_support/testing/time_helpers"
require_relative "../views/core/message_states"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16, 0, 0)
class MessagingGoldenController < ApplicationController; end
MessagingGoldenController.perform_caching = true
Rails.cache = ActiveSupport::Cache::MemoryStore.new
user = User.find_by!(email_address: "david@37signals.com")
other = User.find_by!(email_address: "jz@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
request = ActionDispatch::Request.new(Rack::MockRequest.env_for("http://campfire.test/", "rack.session" => {}))
controller = MessagingGoldenController.new
controller.set_request! request
controller.set_response! ActionDispatch::Response.new
view = controller.view_context
inputs = [
  { markdown_source: "# Release\n\n**Ready** & safe", client_message_id: "ws8bm-markdown" },
  { markdown_source: "| one | two |\n| --- | --- |\n| table | :github: |\n\n- [x] @[David]", client_message_id: "ws8bm-table" },
  { markdown_source: "```ruby\n<script>\n```\n\n:smile: @[Nobody]", client_message_id: "ws8bm-code" },
  { markdown_source: "pinned a message", system_note: true, client_message_id: "ws8bm-note" },
  { body: "<h2>Forwarded</h2><table><tbody><tr><td>kept</td></tr></tbody></table>",
    forwarded_markdown: true, forwarded_at: Time.current, client_message_id: "ws8bm-forward" }
]
rows = inputs.map do |input|
  input = input.merge(forwarded_from_message_id: Message.find_by!(client_message_id: "ws8bm-markdown").id) if input[:forwarded_at]
  message = room.root_messages.create!(input.merge(creator: user))
  Message.preload_rendering_details([message])
  html = [user, other].map do |viewer|
    Current.user = viewer
    MessagingGoldenController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {}).render(
      partial: "messages/message", locals: { message: message })
  end
  raise "Message fragment contains session state" if html.any? { |s| s.include?("authenticity_token") || s.match?(/nonce="[^"]+/) }
  { input: input, message: message_view_facts(view, message), html_by_viewer: html }
ensure
  Current.reset
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], messages: rows) + "\n")
puts "WS8bm fragment oracle: #{rows.size} real Rails messages; 2 viewers through one fragment cache; 0 session-bound values"
