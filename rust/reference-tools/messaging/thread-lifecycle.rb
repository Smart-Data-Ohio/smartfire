# Ordinary creation/lifecycle through the actual Rails controller and publisher.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config["action_dispatch.show_exceptions"] = :all
ApplicationController.allow_forgery_protection = false
room = Room.find(486777696)
david = User.find(127326141)
creator = User.find(149087659)
creator.update_columns(role: :member)
parent = room.root_messages.create!(creator: david, markdown_source: "Lifecycle parent", client_message_id: "lifecycle-parent")
thread = ChannelThread.create!(room:, creator:, parent_message: parent, name: "Lifecycle")
ThreadMembership.join!(thread, creator)
browsers = [david, creator].map do |user|
  request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
  request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! "campfire.test"
  [browser, { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }]
end
frames = []
ActionCable.server.singleton_class.prepend(Module.new do
  define_method(:broadcast) do |stream, payload, **options|
    frames << { stream:, payload: }
    super(stream, payload, **options)
  end
end)
rows = []
base = "/rooms/#{room.id}/threads"
capture = ->(name, viewer, method, path, input = {}) do
  travel_to Time.utc(2026, 3, 2, 16) + rows.size * 10
  frames.clear
  browser, headers = browsers[viewer]
  browser.public_send(method, path, params: input, headers: headers.dup, as: :json)
  raise "wrong method for #{name}" unless browser.request.request_method == method.to_s.upcase
  response = browser.response
  html = response.successful? || response.status == 422
  html &&= response.headers["Content-Type"]&.start_with?("text/html") && !response.body.empty?
  body = html ? ApplicationController.renderer.new(http_host: "campfire.test", https: false).render(
    template: "channel_threads/show", layout: false, assigns: browser.controller.view_assigns) : response.body
  rows << { name:, viewer:, method:, path:, input:, time: Time.current.iso8601, status: response.status,
    cache_control: response.headers["Cache-Control"], content_type: response.headers["Content-Type"], location: response.headers["Location"],
    html:, body:, frames: frames.dup, thread_count: ChannelThread.count,
    threads: ChannelThread.where("id >= ?", thread.id).order(:id).map { |record| {
      id: record.id, name: record.name, creator_id: record.creator_id, parent_message_id: record.parent_message_id,
      auto_archive_after_minutes: record.auto_archive_after_minutes, closed_at: record.closed_at&.iso8601(3),
      locked_at: record.locked_at&.iso8601(3), last_activity_at: record.last_activity_at.iso8601(3), updated_at: record.updated_at.iso8601(3),
      tags: record.tag_names, members: record.memberships.order(:user_id).pluck(:user_id),
      messages: record.messages.order(:id).map { |message| { id: message.id, client_message_id: message.client_message_id, markdown_source: message.markdown_source, body: message.body.body&.to_html } }
    } } }
end
capture.call("new_channel", 0, :get, "#{base}/new.html")
capture.call("nested_create", 0, :post, "#{base}.json", { thread: { name: "Nested start", message: { markdown_source: "First **reply**", client_message_id: "lifecycle-first" } } })
raise rows.last.slice(:name, :status, :body).inspect unless Message.exists?(client_message_id: "lifecycle-first")
created = Message.find_by!(client_message_id: "lifecycle-first").thread
capture.call("retry", 0, :post, "#{base}.json", { thread: { name: "Ignored", auto_archive_after_minutes: 5, message: { markdown_source: "", client_message_id: "lifecycle-first" } } })
capture.call("duplicate_parent", 0, :post, "#{base}.json", { thread: { parent_message_id: parent.id } })
capture.call("invalid_parent", 0, :post, "#{base}.json", { thread: { parent_message_id: Message.find_by!(client_message_id: "lifecycle-first").id } })
capture.call("invalid_initial", 0, :post, "#{base}.json", { thread: { name: "Rollback", message: { markdown_source: "", client_message_id: "lifecycle-invalid" } } })
capture.call("invalid_archive", 0, :post, "#{base}.json", { thread: { name: "Wrong clock", auto_archive_after_minutes: "0" } })
capture.call("rename", 1, :patch, "#{base}/#{thread.id}.json", { thread: { name: "Renamed <&>", auto_archive_after_minutes: "1440" } })
capture.call("tags", 1, :patch, "#{base}/#{thread.id}.json", { thread: { tags: " API, bug, api " } })
capture.call("invalid_tags", 1, :patch, "#{base}/#{thread.id}.json", { thread: { name: "Must roll back", tags: "a,b,c,d,e,f" } })
capture.call("invalid_name", 1, :patch, "#{base}/#{thread.id}.json", { thread: { name: "" } })
capture.call("forbidden_lock", 1, :patch, "#{base}/#{thread.id}.json", { thread: { status: "locked" } })
capture.call("close", 1, :patch, "#{base}/#{thread.id}.json", { thread: { status: "closed" } })
capture.call("reopen_without_join", 0, :patch, "#{base}/#{thread.id}.json", { thread: { status: "active" } })
capture.call("creator_reopen", 1, :patch, "#{base}/#{thread.id}.json", { thread: { status: "active" } })
capture.call("moderator_lock", 0, :patch, "#{base}/#{thread.id}.json", { thread: { status: "locked" } })
capture.call("creator_unlock", 1, :patch, "#{base}/#{thread.id}.json", { thread: { status: "active" } })
capture.call("moderator_unlock", 0, :patch, "#{base}/#{thread.id}.json", { thread: { status: "active" } })
thread.update_columns(last_activity_at: 2.hours.ago, auto_archive_after_minutes: 60)
capture.call("stale_reopen", 1, :patch, "#{base}/#{thread.id}.json", { thread: { status: "active" } })
thread.update_columns(last_activity_at: 2.hours.ago)
capture.call("stale_close", 1, :patch, "#{base}/#{thread.id}.json", { thread: { status: "closed" } })
capture.call("forbidden_delete", 1, :delete, "#{base}/#{thread.id}.json")
capture.call("forbidden_html", 1, :patch, "#{base}/#{thread.id}.html", { thread: { status: "locked" } })
capture.call("invalid_html", 1, :patch, "#{base}/#{thread.id}.html", { thread: { name: "" } })
capture.call("delete", 0, :delete, "#{base}/#{thread.id}.json")
capture.call("create_html", 0, :post, "#{base}.html", { name: "HTML thread" })
last = ChannelThread.order(:id).last
capture.call("update_html", 0, :patch, "#{base}/#{last.id}.html", { name: "HTML renamed" })
capture.call("delete_html", 0, :delete, "#{base}/#{last.id}.html")
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], parent_id: parent.id, thread_id: thread.id, created_id: created.id, rows:) + "\n")
puts "WS8bm thread-lifecycle oracle: #{rows.size} actual Rails actions; creation retries, metadata/tags, lifecycle permissions, rollback rows and delete frames"
