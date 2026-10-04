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
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], rows:) + "\n")
puts "WS8bm client-retries oracle: #{rows.size} scenarios; #{rows.sum { |r| r[:responses].size }} actual Rails requests; root/reply/initial scalar IDs and raw blank-value semantics"
