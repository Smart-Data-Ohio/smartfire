# Request and partial oracles from OUR pinned Rails and a private default seed.
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
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
inputs = [
  { question: "Lunch & <snacks>?", labels: ["Tacos", "Pizza"] },
  { question: "Anonymous?", labels: ["Yes", "No"], anonymous: true, multiple: true },
  { question: "Closed?", labels: ["A", "B"], closes_at: "2026-03-03T16:00:00Z" }
]
polls = inputs.map.with_index do |input, index|
  message = room.root_messages.create!(creator: user, markdown_source: input[:question], client_message_id: "features-#{index}")
  Poll.create_for_message!(message:, labels: input[:labels], anonymous: input[:anonymous] || false, multiple: input[:multiple] || false, closes_at: input[:closes_at])
end
partial = ->(name, locals) { ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(partial: name, locals:) }
html = []
html << { index: 0, html: partial.call("polls/poll", { poll: polls[0] }) }
polls[0].cast_vote!(user, [polls[0].poll_options.first.id])
html << { index: 0, html: partial.call("polls/poll", { poll: polls[0].reload }) }
polls[1].cast_vote!(user, [polls[1].poll_options.first.id])
html << { index: 1, html: partial.call("polls/poll", { poll: polls[1].reload }) }
polls[2].close!
html << { index: 2, html: partial.call("polls/poll", { poll: polls[2].reload }) }
html << { index: 0, error: "This poll allows only one option", html: partial.call("polls/poll", { poll: polls[0].reload, vote_error: "This poll allows only one option" }) }
pin_message = polls.first.message
MessagePin.create!(message: pin_message, room:, pinner: user)
pin_html = { count: partial.call("rooms/pins/count", { room: }), list: partial.call("rooms/pins/list", { room: }), badge: partial.call("messages/pin_badge", { message: pin_message.reload }), index: ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(template: "rooms/pins/index", layout: false, assigns: { room: }) }
MessagePin.find_by!(message: pin_message).unpin!
pin_html[:empty_list] = partial.call("rooms/pins/list", { room: })
pin_html[:hidden_badge] = partial.call("messages/pin_badge", { message: pin_message.reload })
steps = [
  { index: 0, method: "get" }, { index: 1, method: "get" },
  { index: 0, method: "post", option_ids: [polls[0].poll_options.last.id.to_s + "junk", polls[0].poll_options.last.id] },
  { index: 0, method: "post", option_ids: polls[0].poll_options.first.id },
  { index: 0, method: "post", option_ids: polls[0].poll_options.map(&:id) },
  { index: 0, method: "post", option_ids: [polls[1].poll_options.first.id] },
  { index: 0, method: "post" },
  { index: 2, method: "post", option_ids: [polls[2].poll_options.first.id] }
]
steps.each do |step|
  poll = polls.fetch(step[:index])
  path = "/rooms/#{room.id}/polls/#{poll.id}"
  path += "/vote" if step[:method] == "post"
  params = step.key?(:option_ids) ? { option_ids: step[:option_ids] } : {}
  browser.public_send(step[:method], path, params:, headers: headers.dup, as: :json)
  raise "poll routing failed" if browser.response.status >= 400 && ![422].include?(browser.response.status)
  step.merge!(status: browser.response.status, json: JSON.parse(browser.response.body), body: browser.response.body)
end
create_inputs = [
  { poll: { question: "New?", options: [" A ", "", " B "], multiple: "off", anonymous: "1" } },
  { poll: { question: "Invalid?", options: ["Only"] } },
  { poll: { question: "Too long?", options: ["x" * 201, "B"] } },
  { poll: { options: ["A", "B"] } },
  { poll: { question: "Blank?", options: ["A", "B"], closes_at: "unparseable" } },
  { poll: { question: "Past?", options: ["A", "B"], closes_at: "2026-03-01T12:00:00Z" } }
]
creates = create_inputs.map do |input|
  before = Message.count
  browser.post "/rooms/#{room.id}/polls", params: input, headers: headers.dup, as: :json
  raise "create routing failed" unless [201, 422].include?(browser.response.status)
  { input:, status: browser.response.status, json: JSON.parse(browser.response.body), body: browser.response.body, message_delta: Message.count - before }
end
pin_message = polls.first.message
pins = ["post", "post", "delete", "delete"].map do |method|
  before = Message.count
  browser.public_send(method, "/messages/#{pin_message.id}/pin", headers: headers.dup, as: :json)
  raise "pin routing failed" unless [200, 201].include?(browser.response.status)
  { method:, status: browser.response.status, json: JSON.parse(browser.response.body), body: browser.response.body, message_delta: Message.count - before }
end
dates = [
  ["Eastern Time (US & Canada)", "2026-03-08T02:30"],
  ["Eastern Time (US & Canada)", "2026-11-01T01:30"],
  ["Hawaii", "2026-03-03T09:00"], ["Hawaii", "2026-03-03T09:00:12.345Z"],
  ["UTC", "2026-03-03"], ["UTC", "17:30"], ["UTC", ""], ["UTC", "unparseable"], ["Australia/Lord_Howe", "2026-10-04T02:15"], ["Pacific/Apia", "2011-12-30T12:00"]
].map do |zone, raw|
  value = Time.use_zone(zone) { Time.zone.parse(raw) }
  { zone:, raw:, time: value&.utc&.iso8601(3) }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], inputs:, poll_ids: polls.map(&:id), option_ids: polls.map { |poll| poll.poll_options.map(&:id) }, message_ids: polls.map(&:message_id), html:, steps:, creates:, pins:, pin_html:, dates:) + "\n")
puts "WS8bm2 Rails oracle: #{steps.size} poll reads/ballots; #{creates.size} poll creates; #{pins.size} pin writes; #{html.size + pin_html.size} partials; #{dates.size} zone/date probes"
