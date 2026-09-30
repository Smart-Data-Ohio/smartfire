require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = false
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
room = Room.find(486777696)
viewer = User.find(127326141)
thread = ChannelThread.create!(room:, creator: viewer, name: 'Signed attachments')
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
headers = {'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
rows = []
capture = ->(name, method, path, input) do
  browser.public_send(method, path, params: input, headers: headers.dup, as: :json)
  client = input.dig(:message, :client_message_id)
  message = Message.find_by(client_message_id: client)
  blob = message&.attachment&.blob
  rows << {name:, method:, path:, input:, status: browser.response.status, body: browser.response.body,
    location: browser.response.headers['Location'], content_type: browser.response.headers['Content-Type'], cache_control: browser.response.headers['Cache-Control'],
    message: message && {id: message.id, client_message_id: message.client_message_id, markdown_source: message.markdown_source,
      attachment_id: blob&.id, content_type: blob&.content_type, metadata: blob&.metadata}, message_count: Message.count}
  message
end
file = ActiveStorage::Blob.find(13).signed_id
image = ActiveStorage::Blob.find(1).signed_id
root = capture.call('root_file', :post, "/rooms/#{room.id}/messages.turbo_stream", {message: {client_message_id: 'signed-root', markdown_source: 'File caption', attachment: file}})
child = capture.call('thread_file', :post, "/rooms/#{room.id}/threads/#{thread.id}/messages.json", {message: {client_message_id: 'signed-thread', markdown_source: 'Thread caption', attachment: file}})
capture.call('root_image_replace', :patch, "/rooms/#{room.id}/messages/#{root.id}.json", {message: {client_message_id: 'signed-root', attachment: image}})
capture.call('root_null_delete', :patch, "/rooms/#{room.id}/messages/#{root.id}.json", {message: {client_message_id: 'signed-root', attachment: nil}})
capture.call('thread_image_replace', :patch, "/rooms/#{room.id}/threads/#{thread.id}/messages/#{child.id}.json", {message: {client_message_id: 'signed-thread', attachment: image}})
capture.call('thread_empty_delete', :patch, "/rooms/#{room.id}/threads/#{thread.id}/messages/#{child.id}.json", {message: {client_message_id: 'signed-thread', attachment: ''}})
capture.call('tampered_root', :post, "/rooms/#{room.id}/messages.turbo_stream", {message: {client_message_id: 'signed-tampered-root', attachment: file + 'x'}})
capture.call('tampered_thread', :post, "/rooms/#{room.id}/threads/#{thread.id}/messages.json", {message: {client_message_id: 'signed-tampered-thread', attachment: file + 'x'}})
capture.call('forged_column', :post, "/rooms/#{room.id}/threads/#{thread.id}/messages.json", {message: {client_message_id: 'signed-forged', attachment_blob_id: 13, markdown_source: ''}})
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', thread_id: thread.id, rows:) + "\n")
puts "WS8bm signed-attachments oracle: #{rows.size} actual root/thread attach/replace/delete/tamper/forged-column requests"
