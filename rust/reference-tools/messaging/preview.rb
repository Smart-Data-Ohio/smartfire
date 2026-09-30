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
  { source: source, status: browser.response.status, json: JSON.parse(browser.response.body) }
end
raise "Preview wrote messages" unless Message.count == before
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", room_id: room.id, previews: rows) + "\n")
puts "WS8bm preview oracle: #{rows.size} real Rails HTTP responses; 0 messages written"
