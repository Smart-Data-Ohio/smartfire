# The room's selected root page/unread facts from real requests, and the actual show list slot.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.config.hosts.clear
user = User.find_by!(email_address: "david@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
messages = 85.times.map { |index| room.root_messages.create!(creator: user, markdown_source: "Unread #{index}", client_message_id: "unread-#{index}") }
thread = room.channel_threads.create!(creator: user, parent_message: messages.first, name: "Unread thread")
child = thread.messages.create!(room:, creator: user, markdown_source: "Child", client_message_id: "unread-child")
foreign = user.rooms.find_by!(name: "Quiet Corner").root_messages.create!(creator: user, markdown_source: "Foreign", client_message_id: "unread-foreign")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
source = File.read(Rails.root.join("app/views/rooms/show.html.erb"))
slot = source[/^    <% if @unread_divider_message_id.*?^    <% end %>\n/m] or raise "list boundary changed"
member = room.memberships.find_by!(user:)
states = [
  { name: "read_stale_pointer", pointer: messages[79].id },
  { name: "five_unread", pointer: messages[79].id, unread: true },
  { name: "six_unread", pointer: messages[78].id, unread: true },
  { name: "off_page", pointer: messages[10].id, unread: true },
  { name: "around_unread", pointer: messages[10].id, unread: true, anchor: messages[30].id },
  { name: "thread_anchor", pointer: messages[79].id, unread: true, anchor: child.id },
  { name: "foreign_anchor", pointer: messages[79].id, unread: true, anchor: foreign.id },
  { name: "unknown_anchor", pointer: messages[79].id, unread: true, anchor: 0 },
  { name: "no_pointer", unread: true },
]
rows = states.map do |state|
  member.update_columns(unread_at: state[:unread] ? Time.current : nil, last_read_message_id: state[:pointer])
  path = "/rooms/#{room.id}" + (state.key?(:anchor) ? "?message_id=#{state[:anchor]}" : "")
  browser.get(path, headers:)
  raise "room failed: #{browser.response.status}" unless browser.response.successful?
  controller = browser.controller
  facts = controller.view_assigns
  html = controller.render_to_string(inline: slot, layout: false)
  raise "session value in list" if html.include?("authenticity_token") || html.match?(/nonce="[^"]+/)
  state.merge(status: browser.response.status, total_count: browser.response.headers["X-Total-Count"],
    ids: facts.fetch("messages").map(&:id), divider_id: facts["unread_divider_message_id"],
    count: facts["unread_count"] || 0, scroll: facts["scroll_to_unread_divider"], jump_url: facts["jump_to_unread_url"], html:)
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", message_ids: messages.map(&:id), child_id: child.id, foreign_id: foreign.id, rows:) + "\n")
puts "WS8bm room-list oracle: #{rows.size} real Rails room requests; selected roots/unread facts and show list-slot bytes; 0 session-bound values"
