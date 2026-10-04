# Pinned Smartfire oracle. This script never contacts GitHub.
require "json"
require "digest"
require File.join(ENV.fetch("PARITY_WORK", "/work"), "reference-tools/replay_encryption_entropy")
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
PIN = ENV.fetch("PARITY_REFERENCE_SHA")
expected = JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES")))
expected.each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose = false
load Rails.root.join("db/schema.rb")
now = Time.utc(2026, 1, 1, 12)
travel_to(now)
Account.create!(name: "GitHub oracle")
user = User.create!(name: "Oracle", email_address: "oracle@example.test", password: "fixture-password", role: :administrator)
account = ReplayEncryptionEntropy.with("github") do
  GithubConnectedAccount.create!(user:, github_login: "OctoCat", access_token: "fixture-github-access", refresh_token: "fixture-github-refresh", token_source: "app", token_expires_at: now + 3600)
end
row = ActiveRecord::Base.connection.select_one("SELECT * FROM github_connected_accounts WHERE id = #{account.id}")
verifier = Rails.application.message_verifier("github_app_oauth_state")
state = "0" * 32
states = [
  ["valid", verifier.generate(state), state],
  ["wrong_session", verifier.generate(state), "1" * 32],
  ["missing_session", verifier.generate(state), nil],
  ["wrong_session_type", verifier.generate(state), [state]],
  ["wrong_signed_type", verifier.generate([state]), state],
  ["wrong_verifier", Rails.application.message_verifier("slack_oauth_state").generate(state), state],
  ["wrong_purpose", verifier.generate(state, purpose: "wrong"), state],
  ["expired", verifier.generate(state, expires_at: now), state],
  ["fresh_expiring", verifier.generate(state, expires_at: now + 1), state]
].map do |name, signed, stored|
  verified = verifier.verified(signed)
  accepted = verified.is_a?(String) && stored.is_a?(String) && verified.bytesize == stored.bytesize && Rack::Utils.secure_compare(verified, stored)
  { name:, signed:, stored:, accepted: }
end
ENV["GITHUB_APP_CLIENT_ID"] = "fixture-client-id"
ENV["GITHUB_APP_CLIENT_SECRET"] = "fixture-client-secret"
module FakeGitHub
  def start(host, *args, **kwargs)
    raise "Unexpected host #{host}" unless %w[ github.com api.github.com ].include?(host)
    http = Object.new
    def http.post(*args) = Thread.current[:github_response]
    def http.get(*args) = Thread.current[:github_response]
    def http.request(*args) = Thread.current[:github_response]
    yield http
  end
end
Net::HTTP.singleton_class.prepend(FakeGitHub)
def response(status, body, headers = {})
  klass = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s)
  r = klass.new("1.1", status.to_s, "Fixture")
  r.define_singleton_method(:body) { body }
  headers.each { |k,v| r[k] = v }
  Thread.current[:github_response] = r
end
def outcome
  { value: yield }
rescue => error
  { kind: error.class.name, message: error.message }
end
oauth = [[200, '{"access_token":"fixture-access","refresh_token":"fixture-refresh","expires_in":28800}'], [200, '{"error":"bad_verification_code"}'], [200, '{"error":"bad_refresh_token"}'], [401, '{}'], [500, '{}'], [200, '{}'], [200, 'garbage'], [200, '{"access_token":"fixture-access","error":"also-error"}'], [200, '{"access_token":" "}'], [200, 'null'], [200, '[]'], [200, 'false'], [200, '"access_token"']].map do |status, body|
  response(status, body)
  {status:, body:, expected: outcome { Github::App.exchange_code(code: "fixture-code", redirect_uri: "https://app.test/callback") }}
end
write = [[200, '{}'], [201, '{"id":12}'], [204, ''], [200, 'garbage'], [401, '{"message":"secret"}'], [403, '{"message":"Denied @[Alice]\\r\\nTry again"}'], [404, '{}'], [422, '{"message":"Invalid reviewer"}'], [500, '{"message":"secret"}'], [302, '{}'], [403, 'null'], [403, '[]'], [403, '"message"'], [403, '{"message":["@[X]","a\\nb"]}'], [403, '{"message":{"key":"@[X]"}}']].map do |status, body|
  response(status, body)
  { status:, body:, expected: outcome { Github::WriteClient.new(token: "fixture-access").get_user }}
end
login = ['{"login":"octocat"}', '{}', '{"login":null}', '{"login":" "}', '[]', 'null', '"login"', '{"login":["alice","bob"]}', '{"login":{"a":"b"}}'].map do |body|
  response(200, body)
  { status: 200, body:, expected: outcome { Github::WriteClient.authenticated_login("fixture-access") } }
end
read = [[404, "{}", {}], [401, "{}", {}], [403, "{}", {}], [429, "{}", {}], [500, "{}", {}], [200, "garbage", {}], [403, "{}", {"X-RateLimit-Remaining" => "0", "X-RateLimit-Reset" => "1767268800"}], [429, "{}", {"X-RateLimit-Remaining" => "0", "X-RateLimit-Reset" => "bogus"}]].map do |status, body, headers|
  response(status, body, headers)
  pr = Struct.new(:owner,:repo,:number).new("rails","rails",12)
  { status:, body:, headers:, expected: outcome { Github::PullRequestFetcher.new(pr, token: "fixture-workspace-token").send(:get,"pulls/12") } }
end
output = { reference_pin: PIN, reference_hashes: expected, secret_key_base: ENV.fetch("SECRET_KEY_BASE"), now: now.iso8601, row:, plaintext: { access_token: account.access_token, refresh_token: account.refresh_token }, states:, oauth:, write:, login:, read: }
File.write(ENV.fetch("GITHUB_VECTOR_PATH"), JSON.pretty_generate(output) + "\n")
puts "GitHub vectors: #{states.size} state, #{oauth.size} OAuth, #{write.size} REST, #{login.size} login, #{read.size} read, 2 encrypted columns; reference #{PIN}"
