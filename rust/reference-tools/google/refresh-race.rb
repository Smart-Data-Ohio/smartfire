# Two stale model instances finishing in order; no live Google requests.
require 'json'
require 'net/http'
responses = []
http = Object.new
http.define_singleton_method(:post) do |path, body, headers|
  status, payload = responses.shift || raise('unrecorded HTTP request forbidden')
  response = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1', status.to_s, 'fixture')
  response.define_singleton_method(:body) { payload.to_json }
  response
end
Net::HTTP.define_singleton_method(:start) { |*args, **kwargs, &block| block.call(http) }
ENV['GOOGLE_CLIENT_ID'] = 'test-client-id'
ENV['GOOGLE_CLIENT_SECRET'] = 'FAKE-review-client-secret'
now = Time.utc(2026, 9, 30, 12)
Time.define_singleton_method(:current) { now }
user = User.find(127326141)
cases = [
  ['expiry_only', 'access-token', 'access-token', 7200],
  ['access_only', 'access-token', 'latest-access', -3600],
  ['nil_access_unchanged', nil, nil, 7200],
  ['both_changed', 'access-token', 'latest-access', 7200]
].map do |name, original, delayed, expiry|
  GoogleAccount.where(user:).delete_all
  account = GoogleAccount.create!(user:, email: 'fixture@example.test', access_token: original,
    refresh_token: 'refresh-token', access_token_expires_at: now - 1.hour)
  a = GoogleAccount.find(account.id)
  b = GoogleAccount.find(account.id)
  first_response = {access_token:'newer-access',expires_in:3600}
  second_response = {access_token:delayed,expires_in:expiry}
  responses << [200,first_response]
  Google::Client.new(a).refresh_access_token!
  responses << [200,second_response]
  Google::Client.new(b).refresh_access_token!
  {name:,original:,responses:[first_response,second_response],final_token:account.reload.access_token,
    final_expiry:account.access_token_expires_at.iso8601,refresh_token:account.refresh_token}
end
puts JSON.pretty_generate({reference:'d7c7de92',now:now.to_i,cases:})
