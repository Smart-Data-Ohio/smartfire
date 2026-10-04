# Actual HTTP responses from our pinned Rails, over the default parity seed.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16, 0, 0)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear

user = User.find_by!(email_address: "david@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
session = user.sessions.where.not(two_factor_verified_at: nil).first!
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
cookie = "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"

sources = [
  "## Preview\n\n<script>x</script>\n\n- [x] @[David]",
  "| one | two |\n| --- | --- |\n| <b>&</b> | **bold** |",
  ":github: :smile: @[Nobody] [room](/rooms/#{room.id})",
  "[unsafe](javascript:alert(1)) ![image](https://example.test/image.png)",
  "```ruby\n@[David] <script>\n```\n\n~~strike~~",
  "", "é" * 50_000, "é" * 50_001
]
before = Message.count
rows = sources.map do |source|
  browser.post "/rooms/#{room.id}/messages/preview", params: { message: { markdown_source: source } },
    headers: { "Cookie" => cookie }, as: :json
  { source: source, status: browser.response.status, json: JSON.parse(browser.response.body), json_text: browser.response.body }
end
raise "Preview wrote messages" unless Message.count == before
create_inputs = [
  { markdown_source: "" },
  { markdown_source: "é" * 50_001 },
  { markdown_source: "safe", drive_file_ids: "scalar-file-id" },
  { markdown_source: "safe", drive_file_ids: ["invalid!"] },
  { markdown_source: "safe", drive_file_ids: ["\u00a0abcdefghij\u00a0"] },
  { markdown_source: "safe", drive_file_ids: ["abcdefghij", { invalid: "value" }] }
]
creates = create_inputs.map do |input|
  browser.post "/rooms/#{room.id}/messages.json", params: { message: input }, headers: { "Cookie" => cookie }, as: :json
  { input: input, status: browser.response.status, json: JSON.parse(browser.response.body) }
end
raise "Invalid create wrote messages" unless Message.count == before
casts = [nil, false, true, 0, 42, 1.5, "", " text "].map do |value|
  probe = Message.new(client_message_id: value, markdown_source: value)
  { value: value, client_message_id: probe.client_message_id, markdown_source: probe.markdown_source }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], room_id: room.id, previews: rows, invalid_creates: creates, scalar_casts: casts) + "\n")
puts "WS8bm preview oracle: #{rows.size} real Rails HTTP responses; 0 messages written"
puts "WS8bm invalid-create oracle: #{creates.size} real Rails HTTP responses; 0 messages written"
puts "WS8bm scalar-cast oracle: #{casts.size} actual Rails model assignments"
