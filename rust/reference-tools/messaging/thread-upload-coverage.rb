require 'json'
require_relative 'oracle-database'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = false
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
Rails.backtrace_cleaner.remove_silencers!
room = Room.find(486777696)
viewer = User.find(127326141)
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
headers = {'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
rows = []
last_message = Message.maximum(:id)
last_thread = ChannelThread.maximum(:id)
capture = ->(path, input) do
  browser.post(path, params: input, headers: headers.dup, as: :json)
  {path:, input:, status: browser.response.status, body: browser.response.body,
   content_type: browser.response.headers['Content-Type'], cache_control: browser.response.headers['Cache-Control'], location: browser.response.headers['Location']}
end
scenario = MessagingOracleDatabase.scenarios(ARGV.fetch(0))
%w[root reply initial].each do |kind|
  [true, false, 0, 12.5, 'retry-id', '', " \t", "\u00a0"].each_with_index do |client, index|
    scenario.call do
      thread = ChannelThread.create!(room:, creator: User.find(149087659), name: 'Review') if kind == 'reply'
      path = case kind
      when 'root' then "/rooms/#{room.id}/messages.turbo_stream"
      when 'reply' then "/rooms/#{room.id}/threads/#{thread.id}/messages.json"
      else "/rooms/#{room.id}/threads.json"
      end
      input = {message: {client_message_id: client, markdown_source: 'Retry coverage'}}
      input[:thread] = {name: 'Retry coverage'} if kind == 'initial'
      responses = 2.times.map { capture.call(path, input) }
      rows << {name: "#{kind}_client_#{index}", kind:, responses:, now: Time.current.iso8601,
        clients: Message.where('id > ?', last_message).order(:id).pluck(:client_message_id), thread_count: ChannelThread.where('id > ?', last_thread).count}
    end
  end
end
%w[top nested].each do |slot|
  capabilities = {'file' => ActiveStorage::Blob.find(13).signed_id, 'image' => ActiveStorage::Blob.find(1).signed_id,
    'video' => ActiveStorage::Blob.find(9).signed_id, 'bmp' => ActiveStorage::Blob.find(14).signed_id,
    'tampered' => ActiveStorage::Blob.find(13).signed_id + 'x', 'purpose' => ActiveStorage::Blob.find(13).signed_id(purpose: :wrong_attachment),
    'missing_blob' => ActiveStorage::Blob.signed_id_verifier.generate(99_999_999, purpose: 'blob_id'),
    'expired' => ActiveStorage::Blob.find(13).signed_id(expires_in: 1.second), 'missing_file' => ActiveStorage::Blob.find(1).signed_id}
  capabilities.each do |name, attachment|
    scenario.call do
      travel_to Time.utc(2026, 3, 2, 16, 0, name == 'expired' ? 1 : 0)
      message = {client_message_id: "upload-#{slot}-#{name}", markdown_source: 'Initial attachment', attachment:, forward_note: 'Initial note'}
      input = {thread: {name: 'Upload coverage'}}
      slot == 'top' ? input[:message] = message : input[:thread][:message] = message
      missing = name == 'missing_file' ? ActiveStorage::Blob.find(1) : nil
      original = missing&.download
      begin
        missing&.service&.delete(missing.key)
        response = capture.call("/rooms/#{room.id}/threads.json", input)
        saved = Message.find_by(client_message_id: message[:client_message_id])
        rows << {name: "#{slot}_#{name}", kind: 'upload', responses: [response], now: Time.current.iso8601, delete_blob_file: missing&.id,
          clients: Message.where('id > ?', last_message).order(:id).pluck(:client_message_id), thread_count: ChannelThread.where('id > ?', last_thread).count,
          attachment: saved&.attachment&.blob && {id: saved.attachment.blob.id, metadata: saved.attachment.blob.metadata}, forward_note: saved&.forward_note}
      ensure
        missing&.service&.upload(missing.key, StringIO.new(original), checksum: missing.checksum)
      end
    end
  end
  travel_to Time.utc(2026, 3, 2, 16)
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', rows:) + "\n")
puts "WS8bm thread-upload-coverage oracle: #{rows.size} scenarios; #{rows.sum { |r| r[:responses].size }} actual Rails requests; 3 client-id paths, 4 media types, top/nested initial capabilities and rollback"
