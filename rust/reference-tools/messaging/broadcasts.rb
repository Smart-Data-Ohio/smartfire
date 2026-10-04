# Capture the actual publisher's rendered frames; never substitute a renderer or HTML helper.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
frames = []
recorder = Module.new do
  define_method(:broadcast) do |stream, payload, **options|
    frames << { stream:, payload: }
    super(stream, payload, **options)
  end
end
ActionCable.server.singleton_class.prepend(recorder)
user = User.find_by!(email_address: "david@37signals.com")
room = user.rooms.find_by!(name: "All Talk")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
browser.host! "campfire.test:3443"
base = "/rooms/#{room.id}/messages"
steps = []
capture = ->(name, &operation) do
  frames.clear
  operation.call
  raise "#{name} failed: #{browser.response.status}" unless browser.response.successful?
  steps << { name:, status: browser.response.status, frames: frames.dup }
end
capture.call("create") do
  browser.post "#{base}.turbo_stream", params: { message: { markdown_source: "**Broadcast**", client_message_id: "broadcast-root" } }, headers:
end
message = Message.find_by!(client_message_id: "broadcast-root")
travel_to Time.utc(2026, 3, 2, 16, 0, 10)
capture.call("update") do
  browser.patch "#{base}/#{message.id}.json", params: { message: { markdown_source: "## Updated", drive_file_ids: ["abcdefghij"] } }, headers:
end
capture.call("domain_append") { Current.reset; message.reload.broadcast_create }
capture.call("domain_replace") { message.broadcast_stream_final }
reply = room.root_messages.create!(creator: user, markdown_source: "Reply", reply_to_message: message, client_message_id: "broadcast-reply")
thread = ChannelThread.create!(room:, parent_message: message, creator: user, name: "Broadcast thread")
thread_reply = nil
capture.call("indicator") do
  thread_reply = thread.messages.create!(room:, creator: user, markdown_source: "Thread reply", reply_to_message: message, client_message_id: "broadcast-thread-reply")
end
travel_to Time.utc(2026, 3, 2, 16, 0, 20)
capture.call("destroy") { browser.delete "#{base}/#{message.id}.turbo_stream", headers: }
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], base_url: "http://campfire.test:3443",
  message_id: message.id, reply_id: reply.id, thread_id: thread.id, thread_reply_id: thread_reply.id, steps:) + "\n")
puts "WS8bm broadcast oracle: #{steps.size} real Rails writes; #{steps.sum { |step| step[:frames].size }} rendered/channel publisher frames; request port 3443"
