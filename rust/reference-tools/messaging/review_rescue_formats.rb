# PR #191: callback rescue_from, action rescue_from and method-local rescue differ.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
Membership.where(user_id: 127326141, room_id: 654632876).delete_all
user = User.find(127326141)
session = user.sessions.where.not(two_factor_verified_at: nil).first!
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
headers = { 'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'

accepts = ['text/html', 'application/json', 'text/vnd.turbo-stream.html', 'text/javascript', 'application/javascript']
header_names = %w[Content-Type Cache-Control Pragma X-Frame-Options X-Content-Type-Options X-Permitted-Cross-Domain-Policies Referrer-Policy]
requests = [
  ['callback', 'get', '/rooms/654632876/polls/1', {}],
  ['callback', 'get', '/rooms/654632876/pins', {}],
  ['callback', 'get', '/rooms/654632876/files', {}],
  ['callback', 'post', '/rooms/654632876/polls', {}],
  ['callback', 'post', '/rooms/654632876/polls/1/vote', {}],
  ['callback', 'get', '/rooms/486777696/polls/999999999', {}],
  ['callback', 'post', '/messages/935962047/pin', {}],
  ['callback', 'delete', '/messages/935962047/pin', {}],
  ['callback', 'post', '/rooms/654632876/slash_commands', {}],
  ['callback', 'get', '/rooms/654632876/message_links/999999999', {}],
  ['action', 'post', '/saved', { message_id: 935962047 }],
  ['action', 'patch', '/saved/999999999', {}],
  ['action', 'delete', '/saved/999999999', {}],
  ['action', 'post', '/rooms/654632876/scheduled_messages', {}],
  ['action', 'patch', '/scheduled_messages/999999999', {}],
  ['action', 'delete', '/scheduled_messages/999999999', {}],
  ['action', 'post', '/scheduled_messages/999999999/send_now', {}],
  ['action', 'get', '/rooms/486777696/message_links/999999999', {}],
  ['action', 'post', '/rooms/486777696/slash_commands', { thread_id: 999999999 }],
  ['local_rescue', 'get', '/autocompletable/slash_commands?room_id=654632876', {}],
  ['local_rescue', 'get', '/autocompletable/slash_commands?room_id=486777696&thread_id=999999999', {}]
]
controls = ['/rooms/654632876/messages', '/rooms/654632876/messages/999999999',
  '/rooms/654632876/threads/999999999', '/autocompletable/users?room_id=654632876',
  '/users/999999999/card', '/totally_missing_route']
requests += controls.map { |path| ['public_exception', 'get', path, {}] }
rows = requests.flat_map do |kind, method, path, input|
  [false, true].flat_map do |xhr|
    accepts.map do |accept|
      request_headers = headers.merge('Accept' => accept, 'Content-Type' => 'application/json')
      request_headers['X-Requested-With'] = 'XMLHttpRequest' if xhr
      browser.public_send(method, path, params: JSON.generate(input), headers: request_headers)
      response = browser.response
      # The original Astra probe: 18 public exceptions plus 15 callback format requests.
      original_probe = !xhr && ((controls.include?(path) && accepts.first(3).include?(accept)) ||
        requests.first(3).any? { |r| r[2] == path })
      { kind: kind, method: method, path: path, input: input, accept: accept, xhr: xhr,
        original_probe: original_probe, status: response.status,
        headers: header_names.to_h { |name| [name.downcase, response.headers[name]] }, body: response.body }
    end
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(rows) + "\n")
puts "REVIEW Rails rescue format matrix: #{rows.size} responses; #{rows.count { |r| r[:original_probe] }} original probe responses; status, 7 headers and body recorded"
