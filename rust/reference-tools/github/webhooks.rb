# Pinned Rails HTTP oracle: signatures are built at runtime, no GitHub requests.
require "json"
require "digest"
require "net/http"
require "base64"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
module NoWebhookNetwork
  def start(*)
    raise "Unexpected outbound HTTP during webhook ingestion"
  end
end
Net::HTTP.singleton_class.prepend(NoWebhookNetwork)
ActiveJob::Base.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose = false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026, 1, 1, 12)
account = Account.create!(name: "Webhook oracle")
user = User.create!(name: "Oracle", email_address: "oracle@example.test", password: "fixture-password", role: :administrator)
room = Rooms::Closed.create!(name: "Webhook room", creator: user)
message = room.messages.create!(creator: user, body: "Webhook reference")
pr = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 123)
pr.update_columns(head_branch: "shiny", private: false, fetched_at: Time.current)
Github::PullRequestReference.create!(message:, pull_request: pr)
Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 124)
base = { "repository" => { "full_name" => "rails/rails" }, "pull_request" => { "number" => 123, "base" => { "repo" => { "full_name" => "rails/rails" } } } }
cases = []
add = ->(name, method: "POST", event: "pull_request", body: base.to_json, secret: "fixture-webhook-secret", signature: "valid", guid: "fixture-delivery", subscribed: false, duplicate: false) {
  cases << { name:, method:, event:, body:, secret:, signature:, guid:, subscribed:, duplicate: }
}
add.call("valid")
%w[ GET PUT OPTIONS ].each { |method| add.call("unsupported_#{method}", method:, body: "") }
add.call("missing_secret", secret: nil)
add.call("empty_secret", secret: "")
add.call("blank_secret", secret: " \t")
add.call("missing_signature", signature: "missing")
add.call("bad_signature", signature: "bad")
add.call("wrong_body_signature", signature: "other_body")
add.call("uppercase_signature", signature: "uppercase")
add.call("wrong_prefix", signature: "prefix")
add.call("trailing_signature_space", signature: "space")
add.call("missing_delivery", guid: nil)
add.call("blank_delivery", guid: " \t")
add.call("invalid_json", body: "{bad")
add.call("empty_body", body: "")
add.call("whitespace_body", body: " \n\t")
add.call("unknown_event", event: "ping", body: "{}")
add.call("no_event", event: nil)
add.call("duplicate", duplicate: true)
add.call("private", body: base.merge("repository" => { "full_name" => "rails/rails", "private" => true }).to_json)
add.call("public", body: base.merge("repository" => { "full_name" => "rails/rails", "private" => false }).to_json)
add.call("unknown_privacy", body: base.merge("repository" => { "full_name" => "rails/rails", "private" => nil }).to_json)
add.call("string_false_privacy", body: base.merge("repository" => { "full_name" => "rails/rails", "private" => "false" }).to_json)
add.call("unreferenced", body: base.merge("pull_request" => { "number" => 124 }).to_json)
add.call("missing_pr", body: base.merge("pull_request" => { "number" => 999 }).to_json)
add.call("review", event: "pull_request_review")
add.call("issue_comment", event: "issue_comment", body: { repository: { full_name: "rails/rails" }, issue: { number: 123, pull_request: nil } }.to_json)
add.call("plain_issue", event: "issue_comment", body: { repository: { full_name: "rails/rails" }, issue: { number: 123 } }.to_json)
%w[ check_suite check_run ].each do |event|
  add.call(event, event:, body: { "repository" => { "full_name" => "Rails/Rails" }, event => { "pull_requests" => [ { "number" => 123 }, { "number" => 123 }, { "number" => 124 }, {} ] } }.to_json)
end
add.call("status", event: "status", body: { repository: { full_name: "Rails/Rails" }, branches: [ { name: "shiny" }, { name: "shiny" } ] }.to_json)
add.call("other_branch", event: "status", body: { repository: { full_name: "rails/rails" }, branches: [ { name: "other" } ] }.to_json)
add.call("base_repository_precedence", body: base.merge("repository" => { "full_name" => "other/repository" }).to_json)
add.call("fallback_repository", body: base.merge("pull_request" => { "number" => "123" }).to_json)
add.call("subscribed", subscribed: true)
add.call("subscribed_unknown_event", event: "ping", subscribed: true)
add.call("subscribed_comment", event: "issue_comment", subscribed: true, body: { repository: { full_name: "rails/rails" }, issue: { number: 123, pull_request: {} } }.to_json)
add.call("subscribed_duplicate", subscribed: true, duplicate: true)
add.call("invalid_repository", subscribed: true, event: "ping", body: { repository: { full_name: "rails/rails/extra" } }.to_json)
# Successful JSON parsing still preserves Ruby's shape errors; these return Rails' 500 page.
['null', 'false', '5', '"hello"', '[]'].each { |body| add.call("scalar_#{body}", body:) }
add.call("bad_nested_pr", body: { pull_request: 4 }.to_json)
add.call("bad_nested_repository", event: "ping", body: { repository: "invalid" }.to_json)
add.call("bad_check_list", event: "check_run", body: { repository: { full_name: "rails/rails" }, check_run: { pull_requests: 5 } }.to_json)
output = cases.map do |test|
  Github::WebhookDelivery.delete_all
  Github::RepositorySubscription.delete_all
  pr.update_columns(private: false)
  Github::RepositorySubscription.create!(room:, created_by: user, owner: "rails", repo: "rails") if test[:subscribed]
  Github::WebhookDelivery.claim!(test[:guid], event: "first") if test[:duplicate]
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  ENV["GITHUB_WEBHOOK_SECRET"] = test[:secret]
  headers = { "Content-Type" => "application/json" }
  headers["X-GitHub-Event"] = test[:event] if test[:event]
  headers["X-GitHub-Delivery"] = test[:guid] if test[:guid]
  signature = "sha256=#{OpenSSL::HMAC.hexdigest('SHA256', test[:secret] || 'fixture-webhook-secret', test[:body])}"
  signature = case test[:signature]
    when "missing" then nil
    when "bad" then "sha256=deadbeef"
    when "other_body" then "sha256=#{OpenSSL::HMAC.hexdigest('SHA256', test[:secret], test[:body] + ' ')}"
    when "uppercase" then signature.upcase
    when "prefix" then signature.sub('sha256=', 'sha1=')
    when "space" then signature + ' '
    else signature
    end
  headers["X-Hub-Signature-256"] = signature if signature
  session = ActionDispatch::Integration::Session.new(Rails.application)
  session.public_send(test[:method].downcase, '/github/webhooks', params: test[:body], headers:)
  jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.map { |job| job[:job].name }
  test.merge(expected: { status: session.response.status, body: session.response.body, content_type: session.response.headers['Content-Type'], deliveries: Github::WebhookDelivery.count, jobs:, private: pr.reload.private })
end
File.write(ENV.fetch("GITHUB_WEBHOOK_VECTOR_PATH"), JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA"), cases: output }) + "\n")
puts "GitHub webhook Rails oracle: #{output.size} HTTP cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
