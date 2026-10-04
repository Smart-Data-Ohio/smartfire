# Real thread writes and the actual rendered publisher, including retries and rejected writes.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
user = User.find(127326141)
other = User.find(149087659)
room = user.rooms.find(486777696)
parent = room.root_messages.create!(creator: user, markdown_source: "Write parent", client_message_id: "writes-parent")
thread = ChannelThread.create!(room:, creator: other, parent_message: parent, name: "Write thread")
initial = thread.post_message!(creator: other, attributes: { markdown_source: "Original", client_message_id: "writes-original" })
thread.close!
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
frames = []
ActionCable.server.singleton_class.prepend(Module.new do
  define_method(:broadcast) do |stream, payload, **options|
    frames << { stream:, payload: }
    super(stream, payload, **options)
  end
end)
base = "/rooms/#{room.id}/threads/#{thread.id}/messages"
rows = []
capture = ->(name, method, path, input = nil) do
  travel_to Time.utc(2026, 3, 2, 16) + rows.size * 10
  frames.clear
  browser.public_send(method, path, params: input ? { message: input } : {}, headers:, as: :json)
  response = browser.response
  thread.reload
  records = thread.messages.order(:id).map do |message|
    { id: message.id, client_message_id: message.client_message_id, creator_id: message.creator_id,
      markdown_source: message.markdown_source, body: message.body.body&.to_html, edited_at: message.edited_at&.iso8601(3),
      reply_to_message_id: message.reply_to_message_id, reply_target_deleted_at: message.reply_target_deleted_at&.iso8601(3),
      drive_file_ids: message.drive_attachments.map(&:file_id) }
  end
  rows << { name:, method:, path:, input:, time: Time.current.iso8601, status: response.status,
    cache_control: response.headers["Cache-Control"], content_type: response.headers["Content-Type"], location: response.headers["Location"],
    body: response.body, frames: frames.dup, records:, closed: thread.closed?, locked: thread.locked?,
    joined: thread.memberships.exists?(user:), message_count: thread.message_count }
end
capture.call("post", :post, "#{base}.json", { markdown_source: "**Nested**", client_message_id: "writes-post", reply_to_message_id: parent.id, drive_file_ids: ["abcdefghij"] })
posted = Message.find_by!(client_message_id: "writes-post")
capture.call("retry", :post, "#{base}.json", { markdown_source: "Discarded retry", client_message_id: "writes-post", drive_file_ids: "invalid retry" })
capture.call("update", :patch, "#{base}/#{posted.id}.json", { markdown_source: "## Edited", drive_file_ids: ["klmnopqrst"] })
capture.call("identical", :patch, "#{base}/#{posted.id}.json", { markdown_source: "## Edited" })
capture.call("legacy", :patch, "#{base}/#{posted.id}.json", { body: "<div><strong>Legacy</strong></div>" })
capture.call("blank", :patch, "#{base}/#{posted.id}.json", { markdown_source: "" })
capture.call("invalid_drive", :patch, "#{base}/#{posted.id}.json", { markdown_source: "Discarded", drive_file_ids: "scalar" })
capture.call("drive_only", :post, "#{base}.turbo_stream", { client_message_id: "writes-drive", drive_file_ids: ["abcdefghij"] })
reply = thread.messages.create!(room:, creator: user, markdown_source: "Reply", reply_to_message: posted, client_message_id: "writes-reply")
capture.call("destroy", :delete, "#{base}/#{posted.id}.turbo_stream")
capture.call("not_author", :patch, "#{base}/#{initial.id}.json", { markdown_source: "Not allowed" })
thread.lock_conversation!
capture.call("locked_post", :post, "#{base}.json", { markdown_source: "Blocked", client_message_id: "writes-blocked" })
capture.call("locked_update", :patch, "#{base}/#{reply.id}.json", { markdown_source: "Blocked" })
capture.call("locked_delete", :delete, "#{base}/#{reply.id}.json")
note = thread.messages.create!(room:, creator: user, markdown_source: "Immutable", system_note: true, client_message_id: "writes-note")
capture.call("note_update", :patch, "#{base}/#{note.id}.json", { markdown_source: "Blocked" })
capture.call("note_delete", :delete, "#{base}/#{note.id}.json")
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], parent_id: parent.id, thread_id: thread.id,
  initial_id: initial.id, posted_id: posted.id, reply_id: reply.id, note_id: note.id, rows:) + "\n")
puts "WS8bm thread-message write oracle: #{rows.size} actual Rails writes; #{rows.sum { |row| row[:frames].size }} publisher frames; retries, rows, Drive sets, locks and tombstones"
