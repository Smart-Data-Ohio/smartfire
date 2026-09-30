# HTTP paging, validators and implicit formats from our pinned Rails/default seed.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
user = User.find_by!(email_address: "david@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
messages = 85.times.map do |index|
  room.root_messages.create!(creator: user, markdown_source: "Paging #{index}", client_message_id: "paging-#{index}")
end
capture = ->(name, path, extra = {}) do
  browser.get path, headers: headers.merge(extra)
  { name:, path:, status: browser.response.status, etag: browser.response.headers["ETag"],
    last_modified: browser.response.headers["Last-Modified"], cache_control: browser.response.headers["Cache-Control"],
    ids: browser.response.body.scan(/data-message-id="(\d+)"/).flatten.map(&:to_i),
    html: browser.response.successful? && browser.response.media_type == "text/html" ? browser.response.body : nil }
end
base = "/rooms/#{room.id}/messages"
pages = [
  capture.call("last", base),
  capture.call("before", "#{base}?before=#{messages[50].id}"),
  capture.call("after", "#{base}?after=#{messages[40].id}"),
  capture.call("before_wins", "#{base}?before=#{messages[50].id}&after=#{messages[40].id}"),
  capture.call("around_ignored", "#{base}?around=#{messages[40].id}"),
  capture.call("empty", "#{base}?after=#{messages.last.id}"),
  capture.call("blank", "#{base}?before=&after="),
  capture.call("bad_anchor", "#{base}?before=0"),
  capture.call("conditional", base, "If-None-Match" => capture.call("etag", base)[:etag]),
  capture.call("modified_since", base, "If-Modified-Since" => "Mon, 02 Mar 2026 16:00:00 GMT"),
  capture.call("frame", base, "Turbo-Frame" => "messages"),
  capture.call("json", "#{base}.json"),
  capture.call("empty_json", "#{base}.json?after=#{messages.last.id}")
]
controller = MessagesController.new
digest = ActionView::Digestor.digest(name: "messages/index", format: nil, finder: controller.lookup_context)
page = room.root_messages.last_page
related = controller.send(:rendered_related_stamp, page)
pins = controller.send(:rendered_pin_stamp, page)
expanded = ActiveSupport::Cache.expand_cache_key([page, related, pins, MessagesHelper::PRESENTATION_CACHE_VERSION])
formats = %w[html json turbo_stream].flat_map do |format|
  %w[show edit create destroy].map do |action|
    message = messages.first
    path = "#{base}/#{message.id}.#{format}"
    case action
    when "show" then browser.get path, headers:
    when "edit" then browser.get "#{base}/#{message.id}/edit.#{format}", headers:
    when "create" then browser.post "#{base}.#{format}", params: { message: { markdown_source: "format #{format}", client_message_id: "format-#{format}" } }, headers:
    when "destroy"
      message = room.root_messages.create!(creator: user, markdown_source: "destroy #{format}", client_message_id: "destroy-#{format}")
      browser.delete "#{base}/#{message.id}.#{format}", headers:
    end
    { action:, format:, status: browser.response.status, content_type: browser.response.media_type,
      cache_control: browser.response.headers["Cache-Control"],
      removed: action == "destroy" ? !Message.exists?(message.id) : nil,
      body: action == "destroy" && browser.response.successful? ? browser.response.body : nil }
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", message_ids: messages.map(&:id),
  template_digest: digest, expanded_etag: expanded, related_stamp: related, pin_stamp: pins, pages:, formats:) + "\n")
File.write(File.join(File.dirname(ARGV.fetch(0)), "index-template-digest.txt"), digest + "\n")
puts "WS8bm paging oracle: #{pages.size} real Rails page requests; #{formats.size} format requests; template digest #{digest}"
