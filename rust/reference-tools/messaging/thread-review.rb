require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = false
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
room = Room.find(486777696)
viewer = User.find(127326141)
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
headers = {'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
rows = []
capture = ->(name, path, input) do
  browser.post(path, params: input, headers: headers.dup, as: :json)
  {name:, path:, input:, status: browser.response.status, body: browser.response.body,
   content_type: browser.response.headers['Content-Type'], cache_control: browser.response.headers['Cache-Control'], location: browser.response.headers['Location']}
end
scenario = ->(&block) do
  ActiveRecord::Base.transaction do
    block.call
    raise ActiveRecord::Rollback
  end
end
scenario.call do
  thread = ChannelThread.create!(room:, creator: User.find(149087659), name: 'Review')
  input = {message: {client_message_id: true, markdown_source: 'retry me'}}
  responses = 2.times.map { capture.call('boolean_retry', "/rooms/#{room.id}/threads/#{thread.id}/messages.json", input) }
  rows << {name: 'boolean_retry', responses:, clients: thread.messages.order(:id).pluck(:client_message_id), count: thread.messages.count}
end
scenario.call do
  thread = ChannelThread.create!(room:, creator: User.find(149087659), name: 'Review'); thread.close!
  blob = ActiveStorage::Blob.find(1); original = blob.download
  begin
    blob.service.delete(blob.key)
    response = capture.call('missing_file', "/rooms/#{room.id}/threads/#{thread.id}/messages.json", {message: {client_message_id: 'review-missing-file', attachment: blob.signed_id}})
    rows << {name: 'missing_file', responses: [response], state: [thread.messages.count, thread.memberships.exists?(user: viewer), thread.reload.closed_at.present?]}
  ensure
    blob.service.upload(blob.key, StringIO.new(original), checksum: blob.checksum)
  end
end
scenario.call do
  response = capture.call('signed_initial', "/rooms/#{room.id}/threads.json", {thread: {name: 'Signed initial'}, message: {client_message_id: 'review-initial', attachment: ActiveStorage::Blob.find(13).signed_id}})
  message = Message.find_by(client_message_id: 'review-initial')
  rows << {name: 'signed_initial', responses: [response], blob_id: message&.attachment&.blob&.id, count: message&.thread&.messages&.count}
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', rows:) + "\n")
puts "WS8bm thread-review oracle: #{rows.sum { |r| r[:responses].size }} actual Rails requests; boolean retry, failed closed-thread media rollback, signed initial attachment"
