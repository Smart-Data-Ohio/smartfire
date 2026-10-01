require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
user = User.find_by!(email_address: "david@37signals.com")
creator = User.find_by!(email_address: "jason@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
parent = room.root_messages.create!(creator: user, markdown_source: "Membership parent", client_message_id: "membership-parent")
thread = ChannelThread.create!(room:, creator:, parent_message: parent, name: "Membership thread")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
base = "/rooms/#{room.id}/threads/#{thread.id}"
steps = [
  ["invalid", "join", { involvement: "invalid" }],
  ["read_before_join", "read", {}],
  ["join_default", "join", {}],
  ["join_update", "join", { involvement: "everything" }],
  ["invalid_joined", "join", { involvement: ["mentions"] }],
  ["read", "read", {}],
  ["read_again", "read", {}],
  ["leave", "leave", {}],
  ["leave_again", "leave", {}],
  ["join_html", "join", { involvement: "nothing" }],
  ["leave_html", "leave", {}]
].map.with_index do |(name, action, input), index|
  travel_to Time.utc(2026, 3, 2, 16) + index
  if name == "read"
    thread.memberships.find_by!(user:).update_column(:unread_at, Time.current)
  end
  format = name.end_with?("html") ? "html" : "json"
  verb = action == "leave" ? :delete : :post
  # read is PATCH in our route table.
  verb = :patch if action == "read"
  browser.public_send(verb, "#{base}/#{action}.#{format}", params: input, headers:)
  member = thread.memberships.find_by(user:)
  row = member&.slice("id", "involvement", "joined_at", "unread_at", "updated_at")
  %w[joined_at unread_at updated_at].each { |key| row[key] = member.public_send(key)&.utc&.iso8601(3) } if member
  { name:, action:, input:, verb:, format:, time: Time.current.iso8601, status: browser.response.status,
    json: browser.response.media_type == "application/json" && browser.response.body.present? ? JSON.parse(browser.response.body) : nil,
    json_text: browser.response.media_type == "application/json" && browser.response.body.present? ? browser.response.body : nil,
    cache_control: browser.response.headers["Cache-Control"], location: browser.response.headers["Location"],
    membership: row }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", parent_id: parent.id, thread_id: thread.id, steps:) + "\n")
puts "WS8bm thread-membership oracle: #{steps.size} real Rails requests; membership rows and JSON bytes captured"
