# Real root-message requests and saved rows from our pinned Rails/default parity seed.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16, 0, 0)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear

users = %w[david jason].map { |name| User.find_by!(email_address: "#{name}@37signals.com") }
room = users.first.rooms.find_by!(name: "All Talk")
browsers = users.map do |user|
  session = user.sessions.where.not(two_factor_verified_at: nil).first!
  request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
  request.cookie_jar.signed[:session_token] = session.token
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! "campfire.test"
  [browser, { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }]
end
inputs = [
  { markdown_source: "**Before**\n\n@[David]", client_message_id: "root-markdown" },
  { body: "<div>**literal** &amp; <strong>bold</strong></div>", client_message_id: "root-legacy" },
  { markdown_source: "pinned a message", system_note: true, client_message_id: "root-note" }
]
messages = inputs.map { |input| room.root_messages.create!(input.merge(creator: users.first)) }
inline = '<action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" url="https://example.test/legacy" filename="Legacy unfurl" description="Kept alongside Markdown"></action-text-attachment>'
inputs << { body: "<div>Legacy inline</div>#{inline}", client_message_id: "root-inline" }
messages << room.root_messages.create!(inputs.last.merge(creator: users.first))
inputs << { markdown_source: "reply", reply_to_message_id: messages.first.id, reply_notify_author: false, client_message_id: "root-reply" }
messages << room.root_messages.create!(inputs.last.merge(creator: users.first))
thread = ChannelThread.create!(room:, parent_message: messages.first, creator: users.last, name: "Root thread")
MessagePin.create!(message: messages.first, room:, pinner: users.first)
saved = SavedItem.create!(user: users.first, message: messages.first)
[users.first, users.last, users.first].each { |user| Boost.create!(message: messages.first, booster: user, content: "👍") }

actions = messages.flat_map.with_index do |message, index|
  browsers.map.with_index do |(browser, headers), viewer|
    browser.get "/rooms/#{room.id}/messages/#{message.id}/actions", headers:, as: :json
    raise "actions failed: #{browser.response.status}" unless browser.response.successful?
    { index:, viewer:, status: browser.response.status, cache_control: browser.response.headers["Cache-Control"], json: JSON.parse(browser.response.body), json_text: browser.response.body }
  end
end
render_edit = ->(message) { ApplicationController.render(template: "messages/edit", layout: false, assigns: { message:, room: }) }
edits = [0, 1, 3].map { |index| { index:, html: render_edit.call(messages[index]), rendered_body: messages[index].body.to_s } }
actions_menu = ApplicationController.render(partial: "messages/actions")
shows = [0, 1, 2, 4].map { |index| { index:, html: ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(template: "messages/show", layout: false, assigns: { message: messages[index], room: }) } }
steps = [
  { index: 0, input: { markdown_source: "## After\n\n`code`", body: "discarded", client_message_id: "root-changed", drive_file_ids: [" abcdefghij ", "abcdefghij", "klmnopqrst"] } },
  { index: 0, input: { markdown_source: "## After\n\n`code`" } },
  { index: 0, input: { body: "<div>Legacy again</div>" } },
  { index: 0, input: { body: "<div><strong>Legacy again</strong></div>" } },
  { index: 0, input: { markdown_source: "converted", drive_file_ids: [""], reply_to_message_id: messages[1].id, reply_notify_author: "off" } },
  { index: 3, input: { markdown_source: "Inline kept" } }
]
updates = steps.map.with_index do |step, index|
  travel_to Time.utc(2026, 3, 2, 16, 0, 0) + (index + 1) * 10
  message = messages.fetch(step[:index])
  browser, headers = browsers.first
  browser.patch "/rooms/#{room.id}/messages/#{message.id}.json", params: { message: step[:input] }, headers:, as: :json
  raise "update failed: #{browser.response.status}" unless browser.response.successful?
  message.reload
  step.merge(time: Time.current.iso8601, status: browser.response.status, json: JSON.parse(browser.response.body), json_text: browser.response.body, edit_html: render_edit.call(message),
    row: { markdown_source: message.markdown_source, body: message.body.body.to_html,
      plain_text: message.plain_text_body, client_message_id: message.client_message_id,
      edited_at: message.edited_at&.utc&.iso8601(3), reply_to_message_id: message.reply_to_message_id,
      reply_notify_author: message.reply_notify_author?, drive_file_ids: message.drive_attachments.map(&:file_id) })
end
browser, headers = browsers.first
invalid = [{ markdown_source: "" }, { drive_file_ids: "scalar" }].map do |input|
  before = messages.first.reload.attributes
  browser.patch "/rooms/#{room.id}/messages/#{messages.first.id}.json", params: { message: input }, headers:, as: :json
  raise "invalid update changed message" unless messages.first.reload.attributes == before
  { input:, status: browser.response.status, json: JSON.parse(browser.response.body), json_text: browser.response.body }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], inputs:, message_ids: messages.map(&:id),
  thread_id: thread.id, saved_id: saved.id, actions:, edits:, actions_menu:, shows:, updates:, invalid_updates: invalid) + "\n")
puts "WS8bm root oracle: #{actions.size} real Rails actions responses; #{updates.size} updates and saved rows; #{invalid.size} rejected updates; #{edits.size + updates.size} edit forms and 1 actions menu; #{shows.size} standalone messages"
