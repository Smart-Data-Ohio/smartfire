# ChannelThreadsController's actual state listings and standalone reads.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
class GoldenThreadPageController < ChannelThreadsController
  def self.controller_name = "channel_threads"
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
page_env = { http_host: "campfire.test", https: false,
  "HTTP_USER_AGENT" => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
  "rack.session" => {}, "action_dispatch.content_security_policy" => Rails.application.config.content_security_policy,
  "action_dispatch.content_security_policy_nonce_generator" => ->(_request) { "NONCE" } }
viewer = User.find(127326141)
creator = User.find(149087659)
room = viewer.rooms.find(486777696)
parent = room.root_messages.create!(creator: viewer, markdown_source: "Page starter", client_message_id: "pages-parent")
threads = []
threads << ChannelThread.create!(room:, creator:, parent_message: parent, name: "Pages <&> thread")
threads << ChannelThread.create!(room:, creator:, name: "Empty thread")
threads << ChannelThread.create!(room:, creator:, name: "Stale thread", auto_archive_after_minutes: 60)
threads.last.update_columns(last_activity_at: 2.hours.ago)
threads << ChannelThread.create!(room:, creator:, name: "Closed thread")
threads.last.update_columns(closed_at: Time.current)
threads << ChannelThread.create!(room:, creator:, name: "Locked thread")
threads.last.update_columns(closed_at: Time.current, locked_at: Time.current)
threads << ChannelThread.create!(room:, creator:, name: "Work thread", work_status: "planned")
threads.last.update_columns(work_owner_id: viewer.id)
messages = (0...45).map do |index|
  threads.first.messages.create!(room:, creator:, markdown_source: "Page reply #{index}", client_message_id: "pages-#{index}")
end
# Posting sweeps stale siblings. Reset only after the fixture posts, so reads exercise
# time-staleness without a previously persisted closed_at stamp.
threads[2].update_columns(last_activity_at: 2.hours.ago, closed_at: nil)
ThreadMembership.join!(threads.first, creator)
event = WorkThreadEvent.create_for_change!(thread: threads.last, actor: viewer,
  from_status: "planned", to_status: "planned", from_owner: nil, to_owner: viewer, note: "Assigned <&>")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
rows = []
capture = ->(name, path, template, frame = nil) do
  browser.get(path, headers: headers.merge(frame ? { "Turbo-Frame" => frame } : {}))
  response = browser.response
  html = response.successful? && response.headers["Content-Type"]&.start_with?("text/html")
  body = html ? ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(
    template:, layout: false, assigns: browser.controller.view_assigns) : response.body
  raise "session value in page" if html && (body.include?("authenticity_token") || body.match?(/nonce="[^"]+/))
  Current.user = viewer
  full_body = html ? GoldenThreadPageController.renderer.new(page_env).render(template:,
    layout: frame ? "turbo_rails/frame" : "application", assigns: browser.controller.view_assigns) : nil
  rows << { name:, path:, frame:, html:, status: response.status, cache_control: response.headers["Cache-Control"],
    content_type: response.headers["Content-Type"], body: response.successful? ? body : nil,
    title: html ? response.body[/<title>.*?<\/title>/m] : nil, full_body:,
    ids: browser.controller.view_assigns["threads"]&.map(&:id),
    message_ids: browser.controller.view_assigns["messages"]&.map(&:id) }
end
base = "/rooms/#{room.id}/threads"
%w[ active open closed locked all work working done completed unknown ].each do |state|
  capture.call("index_#{state}", "#{base}.json?state=#{state}", "channel_threads/index")
end
capture.call("index_default", "#{base}.json", "channel_threads/index")
capture.call("index_html", base, "channel_threads/index")
capture.call("index_all_html", "#{base}?state=all", "channel_threads/index")
capture.call("index_empty_html", "#{base}?state=done", "channel_threads/index", "thread-pane")
capture.call("index_xml", "#{base}.xml", "channel_threads/index")
capture.call("show_json", "#{base}/#{threads.first.id}.json", "channel_threads/show")
capture.call("show_html", "#{base}/#{threads.first.id}", "channel_threads/show")
capture.call("show_frame", "#{base}/#{threads.first.id}", "channel_threads/show", "thread-pane")
capture.call("show_empty_html", "#{base}/#{threads[1].id}", "channel_threads/show")
capture.call("show_stale_html", "#{base}/#{threads[2].id}", "channel_threads/show")
capture.call("show_closed_html", "#{base}/#{threads[3].id}", "channel_threads/show")
capture.call("show_locked_html", "#{base}/#{threads[4].id}", "channel_threads/show")
capture.call("show_empty", "#{base}/#{threads[1].id}.json", "channel_threads/show")
capture.call("show_locked", "#{base}/#{threads[4].id}.json", "channel_threads/show")
capture.call("show_work", "#{base}/#{threads[5].id}.json", "channel_threads/show")
capture.call("show_xml", "#{base}/#{threads.first.id}.xml", "channel_threads/show")
parent.destroy!
capture.call("show_deleted_parent", "#{base}/#{threads.first.id}.json", "channel_threads/show")
capture.call("show_deleted_parent_html", "#{base}/#{threads.first.id}", "channel_threads/show")
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", parent_id: parent.id,
  thread_ids: threads.map(&:id), message_ids: messages.map(&:id), work_event_id: event.id, rows:,
  brand_icon_names: Icons.client_icon_names,
  viewer: {theme: viewer.theme, text_size: viewer.text_size, time_zone: viewer.time_zone,
    time_zone_explicit: viewer.time_zone_explicit, tour_completed: viewer.tour_completed_at.present?,
    voice_mode: viewer.voice_mode, push_to_talk_key: viewer.push_to_talk_key}) + "\n")
puts "WS8bm thread-pages oracle: #{rows.size} actual Rails requests; state lists, standalone HTML/JSON, latest replies and deleted starter"
