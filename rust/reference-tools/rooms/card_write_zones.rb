# Real request responses and shared broadcasts; Rails alone supplies expected HTML.
require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
viewer = User.find(127326141)
room = Room.find(486777696)
pull_request = Github::PullRequest.find(1)
pull_request.update_columns(github_updated_at: Time.utc(2026, 7, 1, 12))
[["spring", "2026-03-08T06:30:00Z", "2026-03-08T07:30:00Z"],
 ["fall", "2026-11-01T05:30:00Z", "2026-11-01T06:30:00Z"]].each_with_index do |(name, starts, ends), index|
  event = Event.new(id: 8000000601 + index, room:, organizer: viewer, title: "Zone #{name}",
    starts_at: Time.iso8601(starts), ends_at: Time.iso8601(ends), time_zone: "UTC")
  Event.insert_all!([event.attributes.except("created_at", "updated_at").merge("created_at" => Time.current, "updated_at" => Time.current)])
  message = room.root_messages.create_with_attachment!(id: 9000000601 + index, creator: viewer,
    client_message_id: "ws8br-zone-#{name}", markdown_source: "Zone #{name}")
  EventReference.create!(event_id: event.id, message_id: message.id)
end

def container(body, id)
  offset = body.index(%{<div id="#{id}"}) or raise "missing #{id}"
  depth = 0
  body[offset..].to_enum(:scan, %r{</?div\b[^>]*>}).each do
    match = Regexp.last_match
    depth += match[0].start_with?("</") ? -1 : 1
    return body[offset, match.end(0)] if depth.zero?
  end
  raise "unclosed #{id}"
end

request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
cookie = request.cookie_jar[:session_token]
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
browser.cookies[:session_token] = cookie
other = User.find(149087659)
other.update_columns(time_zone: "Asia/Kolkata")
other_session = other.sessions.first || other.sessions.create!
request.cookie_jar.signed[:session_token] = other_session.token
other_browser = ActionDispatch::Integration::Session.new(Rails.application)
other_browser.host! "campfire.test"
other_browser.cookies[:session_token] = request.cookie_jar[:session_token]
frames = []
ActionCable.server.define_singleton_method(:broadcast) { |stream, payload, **options| frames << { stream:, payload: } }
zones = ["Hawaii", "Eastern Time (US & Canada)", "Asia/Kolkata", "Kathmandu", "Sydney", "Adelaide", "UTC", nil, "", "Not a real zone"]
cases = []
zones.each_with_index do |zone, index|
  viewer.update_columns(time_zone: zone)
  browser.get("/rooms/#{room.id}")
  token = Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
  headers = { "Accept" => "text/vnd.turbo-stream.html", "X-CSRF-Token" => token }
  client_id = "ws8br-zone-create-#{index}"
  source = "/rooms/#{room.id}/events/8000000601\n/rooms/#{room.id}/events/8000000602"
  target = "event_cards_message_#{client_id}"
  frames.clear
  browser.post("/rooms/#{room.id}/messages", params: { message: { client_message_id: client_id, markdown_source: source } }, headers:)
  raise "create failed #{browser.response.status}" unless browser.response.status == 200
  html = container(browser.response.body, target)
  broadcast = frames.select { |f| f[:payload].is_a?(String) && f[:payload].include?(%{id="#{target}"}) }
  raise "expected one append" unless broadcast.one?
  message_id = Message.find_by!(room:, client_message_id: client_id).id
  cases << { kind: "message_create", zone:, path: "/rooms/#{room.id}/messages", target:, client_id:, message_id:, source:, status: browser.response.status, html:, broadcast_html: container(broadcast.first[:payload], target) }
end
event = Event.find(8000000601)
target = "event_cards_message_ws8br-zone-spring"
zones.each_with_index do |zone, index|
  viewer.update_columns(time_zone: zone)
  browser.get("/rooms/#{room.id}")
  token = Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
  frames.clear
  title = "Zone edit #{index}"
  browser.patch("/rooms/#{room.id}/events/#{event.id}", params: { event: { title:, starts_at: "2026-03-08T06:30", ends_at: "2026-03-08T07:30", time_zone: "UTC" } }, headers: { "X-CSRF-Token" => token })
  raise "edit failed #{browser.response.status}" unless browser.response.status == 302
  matching = frames.select { |f| f[:payload].is_a?(String) && f[:payload].include?(%{target="#{target}"}) }
  raise "expected one shared replacement" unless matching.one?
  other_browser.get("/rooms/#{room.id}")
  raise "other viewer reload failed" unless other_browser.response.status == 200
  reload_other_html = container(other_browser.response.body, target)
  cases << { reload_other_zone: "Asia/Kolkata", reload_other_html:, kind: "event_edit", zone:, path: "/rooms/#{room.id}/events/#{event.id}", title:, status: browser.response.status, frame: matching.first }
end
frames.clear
Time.use_zone("UTC") { event.reload.update!(title: "Background UTC") }
background = frames.select { |f| f[:payload].is_a?(String) && f[:payload].include?(%{target="#{target}"}) }
raise "expected one background replacement" unless background.one?
message = pull_request.send(:referencing_messages).where(thread_id: nil).first!
target = "github_pr_cards_message_#{message.client_message_id}"
zones.each do |zone|
  viewer.update_columns(time_zone: zone)
  ["/rooms/#{message.room_id}", "/rooms/#{message.room_id}/refresh?since=0", "/rooms/#{message.room_id}/messages", "/rooms/#{message.room_id}/messages/#{message.id}"].each do |path|
    Rails.cache.clear
    browser.get(path, headers: { "Accept" => path.include?("refresh") ? "text/vnd.turbo-stream.html" : "text/html" })
    raise "GitHub read failed #{path}: #{browser.response.status}" unless browser.response.status == 200
    cases << { kind: "github_message", zone:, path:, target:, status: browser.response.status, html: container(browser.response.body, target) }
  end
end
sources = %w[app/controllers/concerns/set_time_zone.rb app/controllers/messages_controller.rb app/controllers/rooms/events_controller.rb app/models/event/channel_timeline.rb app/views/rooms/events/_card.html.erb app/views/rooms/events/_cards.html.erb app/views/github/pull_requests/_card.html.erb app/helpers/github/pull_requests_helper.rb].to_h { |path| [path, Digest::SHA256.file(Rails.root.join(path)).hexdigest] }
puts JSON.generate({ reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], sources:, cases:, background: background.first })
