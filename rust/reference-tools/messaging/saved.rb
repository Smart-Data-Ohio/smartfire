require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
user = User.find_by!(email_address: "david@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
message = room.root_messages.create!(creator: user, markdown_source: "Saved <message> & example", client_message_id: "saved-example")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
item = SavedItem.create!(user:, message:)
html = []
Current.user = user
%w[UTC Hawaii].each do |zone|
  user.update!(time_zone: zone)
  Time.use_zone(zone) do
    [{ status: "in_progress", remind_at: nil, reminded_at: nil },
     { status: "in_progress", remind_at: Time.utc(2026, 3, 3, 17), reminded_at: nil },
     { status: "done", remind_at: Time.utc(2026, 3, 3, 17), reminded_at: Time.utc(2026, 3, 3, 17) }].each do |state|
      item.update_columns(state)
      rendered = ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(partial: "saved_items/item", locals: { saved_item: item.reload, status_filter: "all" })
      html << { zone:, state: state.transform_values { |v| v.respond_to?(:iso8601) ? v.utc.iso8601(3) : v }, html: rendered }
    end
  end
end
item.update_columns(status: "in_progress", remind_at: nil, reminded_at: nil)
empty = ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(template: "saved_items/index", layout: false, assigns: { saved_items: [], status_filter: "all" })
steps = []
call = ->(method, path, input) {
  browser.public_send(method, path, params: input, headers: headers.dup, as: :json)
  raise "saved routing failed" unless [200, 201, 204, 422].include?(browser.response.status)
  steps << { method:, path:, input:, status: browser.response.status, body: browser.response.body,
             cache_control: browser.response.headers["Cache-Control"], pragma: browser.response.headers["Pragma"] }
}
call.call(:post, "/saved", { message_id: message.id, saved_item: { remind_at: "2026-03-02T17:00:00.123Z" } })
id = JSON.parse(steps.last[:body]).fetch("id")
call.call(:post, "/saved", { message_id: message.id, remind_at: "2026-03-02T18:00:00Z" })
call.call(:patch, "/saved/#{id}", { saved_item: { status: "done" } })
call.call(:patch, "/saved/#{id}", { saved_item: { status: "in_progress" } })
call.call(:patch, "/saved/#{id}", { saved_item: { status: "archived" } })
call.call(:post, "/saved", { message_id: message.id, saved_item: { remind_at: "not a time" } })
call.call(:post, "/saved", { message_id: message.id, saved_item: { remind_at: "2026-03-02T15:59:00Z" } })
call.call(:post, "/saved", { message_id: message.id })
call.call(:delete, "/saved/#{id}", {})
call.call(:post, "/saved", { message_id: message.id })
call.call(:delete, "/saved/#{JSON.parse(steps.last[:body]).fetch('id')}", {})
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], message_id: message.id, steps:, html:, empty:) + "\n")
puts "WS8bm2 saved Rails oracle: #{steps.size} HTTP responses; #{html.size} item partials; 1 empty page"
