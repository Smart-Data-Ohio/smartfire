require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
user = User.find_by!(email_address: "david@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
draft = ScheduledMessage.create!(user:, room:, markdown_source: "Draft <body> & example", send_at: Time.utc(2026, 3, 2, 17))
Current.user = user
html = []
%w[UTC Hawaii].each do |zone|
  user.update!(time_zone: zone)
  Time.use_zone(zone) do
    ["pending", "stranded", "sent", "dropped"].each do |state|
      draft.update_columns(sent_at: state == "sent" ? Time.utc(2026,3,2,16) : nil, dropped_at: state == "dropped" ? Time.utc(2026,3,2,16) : nil)
      locals = { scheduled: draft.reload }
      locals[:stranded] = true if state == "stranded"
      name = %w[sent dropped].include?(state) ? "scheduled_messages/past_item" : "scheduled_messages/item"
      html << { zone:, state:, html: ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(partial: name, locals:) }
    end
  end
end
user.update!(time_zone: "UTC")
draft.update_columns(sent_at: nil, dropped_at: nil)
empty = ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(template: "scheduled_messages/index", layout: false, assigns: { upcoming: [], stranded: [], past: [] })
composer = [nil, ChannelThread.new(id: 777, room:)].map do |thread|
  { thread_id: thread&.id, html: ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(partial: "scheduled_messages/composer_button", locals: { room:, thread: }) }
end
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
steps = []
call = ->(method, path, input, zone = "UTC") {
  user.update_columns(time_zone: zone)
  browser.public_send(method, path, params: input, headers: headers.dup, as: :json)
  raise "scheduled routing failed" unless [200, 201, 202, 204, 409, 422].include?(browser.response.status)
  steps << { method:, path:, input:, zone:, status: browser.response.status, body: browser.response.body,
             cache_control: browser.response.headers["Cache-Control"], pragma: browser.response.headers["Pragma"] }
}
path = "/rooms/#{room.id}/scheduled_messages"
call.call(:post, path, { scheduled_message: { markdown_source: "Morning!", send_at: "2026-03-02T17:00:00Z" } })
id = JSON.parse(steps.last[:body]).fetch("id")
call.call(:post, path, { scheduled_message: { markdown_source: "Late", send_at: "2026-03-02T15:00:00Z" } })
call.call(:post, path, { scheduled_message: { markdown_source: "Bad time", send_at: "not a time" } })
call.call(:patch, "/scheduled_messages/#{id}", { scheduled_message: { markdown_source: "Sooner!", send_at: "2026-03-02T18:00" } })
call.call(:patch, "/scheduled_messages/#{id}", { scheduled_message: { send_at: "2026-03-02T15:00:00Z" } })
ScheduledMessage.find(id).update_columns(claimed_at: Time.utc(2026,3,2,16))
call.call(:patch, "/scheduled_messages/#{id}", { scheduled_message: { markdown_source: "Busy" } })
call.call(:post, "/scheduled_messages/#{id}/send_now", {})
ScheduledMessage.find(id).update_columns(claimed_at: Time.utc(2026,3,2,15,54))
call.call(:patch, "/scheduled_messages/#{id}", { scheduled_message: { markdown_source: "Ready" } })
call.call(:post, "/scheduled_messages/#{id}/send_now", {})
call.call(:delete, "/scheduled_messages/#{draft.id}", {})
call.call(:post, path, { scheduled_message: { markdown_source: "Hawaii", send_at: "2026-03-03T09:00" } }, "Hawaii")
call.call(:post, path, { scheduled_message: { markdown_source: "Gap", send_at: "2026-03-08T02:30" } }, "Eastern Time (US & Canada)")
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", draft_id: draft.id, steps:, html:, empty:, composer:) + "\n")
puts "WS8bm2 scheduled Rails oracle: #{steps.size} HTTP responses; #{html.size} row partials; 1 empty page; #{composer.size} composer controls"
