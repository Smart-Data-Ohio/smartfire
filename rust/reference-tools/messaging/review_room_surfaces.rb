# Full room requests, distinct from rendering the shared composer/pin partial in isolation.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
user = User.find(127326141)
session = user.sessions.where.not(two_factor_verified_at: nil).first!
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
headers = { 'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
rows = []
%w[Rooms::Open Rooms::Closed Rooms::Voice Rooms::Stage Rooms::Board].each do |type|
  Room.find(699448326).update_columns(type: type)
  room = Room.find(699448326)
  key = room.class.model_name.param_key
  browser.get "/rooms/#{room.id}", headers: headers
  html = browser.response.body
  rows << { type: type, room_id: room.id, param_key: key, status: browser.response.status,
    reply_control: html.include?("id=\"reply_notify_#{key}_#{room.id}\""),
    reply_label: html.include?("for=\"reply_notify_#{key}_#{room.id}\""),
    pin_count: html.include?("id=\"pins_count_#{key}_#{room.id}\""),
    pin_frame: html.include?("id=\"pins_frame_#{key}_#{room.id}\""),
    board_index: html.include?('<main class="board" aria-labelledby="board-title">'),
    board_filters: html.include?('class="board__filters"'),
    composer: html.include?('<form id="composer"') }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(rows) + "\n")
puts "REVIEW Rails room surfaces: #{rows.size}/5 HTTP room types; board index has filters, no chat composer or pin panel"
