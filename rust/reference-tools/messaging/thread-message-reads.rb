# Actual nested reads: scoped cursors, empty pages, formats, viewer payloads and redirects.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
user = User.find(127326141)
room = user.rooms.find(486777696)
parent = room.root_messages.create!(creator: user, markdown_source: "Thread parent", client_message_id: "reads-parent")
thread = ChannelThread.create!(room:, creator: user, parent_message: parent, name: "Read thread")
empty = ChannelThread.create!(room:, creator: user, name: "Empty thread")
messages = (0...45).map { |index| thread.messages.create!(room:, creator: user, markdown_source: "Thread #{index}", client_message_id: "reads-#{index}") }
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
base = "/rooms/#{room.id}/threads/#{thread.id}/messages"
rows = []
capture = ->(name, path, frame = nil) do
  browser.get(path, headers: headers.merge(frame ? { "Turbo-Frame" => frame } : {}))
  response = browser.response
  json = response.headers["Content-Type"]&.start_with?("application/json")
  html = response.successful? && response.headers["Content-Type"]&.start_with?("text/html")
  # The owned index template is stable; the authenticated application/frame layout contains
  # real per-session tokens and is checked by its owner, never normalized into a golden.
  body = html ? ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(
    template: "channel_thread_messages/index", layout: false, assigns: browser.controller.view_assigns) : response.body
  raise "session value in index" if html && (body.include?("authenticity_token") || body.match?(/nonce="[^"]+/))
  rows << { name:, path:, frame:, html:, status: response.status, cache_control: response.headers["Cache-Control"],
    content_type: response.headers["Content-Type"], location: response.headers["Location"],
    body: response.successful? || response.redirect? ? body : nil,
    ids: browser.controller.view_assigns["messages"]&.map(&:id), json: json ? JSON.parse(response.body) : nil }
end
capture.call("index_html", base)
capture.call("index_frame", base, "thread-pane")
capture.call("index_json", "#{base}.json")
capture.call("before", "#{base}.json?before=#{messages[10].id}")
capture.call("after", "#{base}.json?after=#{messages[35].id}")
capture.call("before_priority", "#{base}.json?before=#{messages[10].id}&after=#{messages[35].id}")
capture.call("around_ignored", "#{base}.json?around=#{messages[0].id}")
capture.call("empty_after_json", "#{base}.json?after=#{messages.last.id}")
capture.call("empty_before_html", "#{base}?before=#{messages.first.id}")
capture.call("empty_thread_json", "/rooms/#{room.id}/threads/#{empty.id}/messages.json")
capture.call("empty_thread_html", "/rooms/#{room.id}/threads/#{empty.id}/messages")
capture.call("show_html", "#{base}/#{messages[10].id}")
capture.call("show_json", "#{base}/#{messages[10].id}.json")
capture.call("actions_html", "#{base}/#{messages[10].id}/actions")
capture.call("index_xml", "#{base}.xml")
capture.call("show_xml", "#{base}/#{messages[10].id}.xml")
thread.update_columns(locked_at: Time.current)
capture.call("locked_actions", "#{base}/#{messages[10].id}/actions.json")
capture.call("locked_show", "#{base}/#{messages[10].id}.json")
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], parent_id: parent.id, thread_id: thread.id,
  empty_id: empty.id, message_ids: messages.map(&:id), rows:) + "\n")
puts "WS8bm thread-message read oracle: #{rows.size} actual Rails requests; scoped pages, empty formats, raw JSON/actions/HTML and locked reads"
