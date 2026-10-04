# Acceptance gaps from test/models/google/client_test.rb. No live HTTP is allowed.
require 'json'
require 'net/http'
ENV['GOOGLE_CLIENT_ID'] = 'test-client-id'
ENV['GOOGLE_CLIENT_SECRET'] = 'FAKE-google-client-secret'
now = Time.utc(2026, 9, 30, 12)
Time.define_singleton_method(:current) { now }
responses = []
calls = []
transport = nil
http = Object.new
http.define_singleton_method(:method_missing) do |method, path, *args|
  headers = args.last.is_a?(Hash) ? args.last : {}
  calls << {method: method.to_s.upcase, path: path, body: %i[get delete].include?(method) ? '' : args.first.to_s,
    content_type: headers['Content-Type'], access_token: headers['Authorization']&.delete_prefix('Bearer ')}
  raise transport.new('disposable transport detail') if transport
  status, body = responses.shift || raise('Unrecorded Google HTTP is prohibited')
  response = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1', status.to_s, 'fixture')
  response.define_singleton_method(:body) { body }
  response
end
Net::HTTP.define_singleton_method(:start) do |host, *args, **kwargs, &block|
  raise "Unexpected Google host: #{host}" unless %w[www.googleapis.com oauth2.googleapis.com].include?(host)
  block.call(http)
end
refresh = {access_token: 'refreshed-access-token', expires_in: 3600}.to_json
cases = []
add = ->(name, operation, answers, extra = {}) { cases << {name: name, operation: operation, responses: answers, **extra} }
[[404, '{}'], [410, '{}'], [409, '{}'], [500, '{}'], [429, '{}'], [403, '{oops'], [403, '{}']].each do |status, body|
  add.call("calendar_#{status}_#{body == '{oops' ? 'malformed' : 'empty'}", 'calendar', [[status, body]])
end
%w[rateLimitExceeded userRateLimitExceeded quotaExceeded forbidden].each do |reason|
  add.call("calendar_403_#{reason}", 'calendar', [[403, {error: {errors: [{reason: reason}]}}.to_json]])
end
add.call('delete_gone', 'delete', [[410, '{}']])
add.call('401_survives_refresh', 'calendar', [[401, '{}'], [200, refresh], [401, '{}']])
add.call('refresh_429', 'calendar', [[429, '{}']], expired: true)
add.call('refresh_503', 'calendar', [[503, '{}']], expired: true)
[[403, '{}'], [404, '{}'], [429, '{}']].each { |status, body| add.call("drive_#{status}", 'drive', [[status, body]]) }
add.call('drive_expired', 'drive', [[200, refresh], [200, {id: '1AbcDefGhIjKlMnOpQrSt', name: 'Fixture'}.to_json]], expired: true)
add.call('list_recent', 'list', [[200, {files: []}.to_json]], query: '')
add.call('list_blank', 'list', [[200, {files: []}.to_json]], query: '   ')
add.call('list_expired', 'list', [[200, refresh], [200, {files: []}.to_json]], query: '', expired: true)
add.call('list_quota', 'list', [[403, {error: {errors: [{reason: 'rateLimitExceeded'}]}}.to_json]], query: '')
add.call('list_pages_cap', 'pages', (1..Google::Client::LIST_MAX_PAGES).map { |n| [200, {items: [{id: "page-#{n}"}], nextPageToken: "next-#{n}"}.to_json] })
add.call('list_events_429', 'pages', [[429, '{}']])
add.call('list_events_quota', 'pages', [[403, {error: {errors: [{reason: 'quotaExceeded'}]}}.to_json]])
add.call('list_events_revoked', 'pages', [[401, '{}'], [400, {error: 'invalid_grant'}.to_json]])
add.call('exchange_success', 'exchange', [[200, {access_token: 'disposable-exchange-token', expires_in: 3600}.to_json]])
add.call('exchange_failure', 'exchange', [[400, {error: 'invalid_grant'}.to_json]])
[200, 204, 400, 403, 429, 503].each { |status| add.call("revoke_#{status}", 'revoke', [[status, '{}']]) }
{'open_timeout' => Net::OpenTimeout, 'read_timeout' => Net::ReadTimeout, 'connection_refused' => Errno::ECONNREFUSED, 'connection_reset' => Errno::ECONNRESET, 'dropped_connection' => EOFError}.each do |name, klass|
  add.call(name, 'calendar', [], transport: klass.name)
end
add.call('drive_timeout', 'drive', [], transport: 'Net::OpenTimeout')
add.call('revoke_timeout', 'revoke', [], transport: 'Net::OpenTimeout')
rows = cases.map do |spec|
  responses.replace(spec[:responses].map(&:dup)); calls.clear
  transport = spec[:transport] && Object.const_get(spec[:transport])
  credentials = Google::Client::SnapshotCredentials.new(access_token: 'access-token', refresh_token: 'refresh-token', access_token_expires_at: spec[:expired] ? now - 3600 : now + 3600)
  disconnected_reason = nil
  credentials.define_singleton_method(:mark_disconnected!) { |reason| disconnected_reason = reason; super(reason) }
  client = Google::Client.new(credentials)
  begin
    value = case spec[:operation]
    when 'calendar' then client.insert_event({})
    when 'delete' then client.delete_event('gone-id')
    when 'drive' then client.drive_file('1AbcDefGhIjKlMnOpQrSt')
    when 'list' then client.list_drive_files(query: spec[:query])
    when 'pages' then client.list_events(time_min: now - 3600, time_max: now + 3600)
    when 'exchange' then Google::Client.exchange_code(code: 'disposable-code', redirect_uri: 'http://test.host/google/callback')
    when 'revoke' then Google::Client.revoke_token('disposable-revoke-token')
    end
    error = nil
  rescue Google::Client::Error => e
    value = nil; error = {class: e.class.name, message: e.message, unavailable: e.is_a?(Google::Client::Unavailable)}
  end
  raise "Unused responses in #{spec[:name]}" unless responses.empty?
  {spec: spec, requests: calls.dup, result: value, error: error, credentials: {access_token: credentials.access_token, expires_at: credentials.access_token_expires_at&.to_i, disconnected_reason: disconnected_reason}}
end
configuration = [['test-client-id', 'secret'], ['', 'secret'], ['test-client-id', ''], [' ', 'secret'], ['test-client-id', ' ']].map do |id, secret|
  ENV['GOOGLE_CLIENT_ID'] = id; ENV['GOOGLE_CLIENT_SECRET'] = secret
  {client_id: id, client_secret: secret, configured: Google::Client.configured?}
end
warn "Pinned Rails Google client acceptance: #{rows.size} recorded cases; #{configuration.size} credential combinations; no Google network"
puts JSON.pretty_generate(reference: 'd7c7de92', now: now.to_i, rows: rows, configuration: configuration)
