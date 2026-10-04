# Fetch persistence oracle; fake only the fixed GitHub API host, never the open network.
require "json"
require "digest"
require "net/http"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ActiveJob::Base.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose = false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026, 1, 1, 12)
Account.create!(name: "Fetch oracle")
user = User.create!(name: "Oracle", email_address: "oracle@example.test", password: "fixture-password", role: :administrator)
room = Rooms::Closed.create!(name: "Fetch room", creator: user)
message = room.messages.create!(creator: user, body: "Discussion")
thread = ChannelThread.create!(room:, creator: user, parent_message: message)
pr = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 123)
data = { "title" => "Add shiny things", "user" => { "login" => "dhh", "avatar_url" => "https://example.test/avatar.png" }, "state" => "open", "draft" => false, "merged_at" => nil, "base" => { "ref" => "main", "repo" => { "private" => false } }, "head" => { "ref" => "shiny", "sha" => "abc" }, "html_url" => "https://github.com/rails/rails/pull/123", "updated_at" => "2026-01-01T00:00:00Z", "changed_files" => 2 }
default = { pr: data, reviews: [], runs: { "check_runs" => [] }, status: { "state" => "pending", "total_count" => 0 }, files: [] }
cases = []
add = ->(name, **changes) { cases << default.merge(name:, **changes) }
add.call("public_success", reviews: [{ state: "APPROVED", submitted_at: "2026-01-01T00:00:00Z", user: { id: 1 } }], status: { state: "success", total_count: 1 })
add.call("private", pr: data.deep_merge("base" => { "repo" => { "private" => true } }))
add.call("unknown_privacy", pr: data.merge("base" => { "ref" => "main" }))
add.call("string_false_privacy", pr: data.deep_merge("base" => { "repo" => { "private" => "false" } }))
add.call("empty_privacy", pr: data.deep_merge("base" => { "repo" => { "private" => "" } }))
add.call("merged", pr: data.merge("state" => "closed", "merged_at" => "2026-01-01T00:00:00Z", "draft" => true))
add.call("closed", pr: data.merge("state" => "closed", "draft" => true))
add.call("draft_approved", pr: data.merge("draft" => true), reviews: [{ state: "APPROVED", user: { id: 1 } }])
add.call("draft_review_required", pr: data.merge("draft" => true))
add.call("truthy_draft", pr: data.merge("draft" => "false"))
add.call("latest_review", reviews: [{ state: "CHANGES_REQUESTED", submitted_at: "2025", user: { id: 1 } }, { state: "APPROVED", submitted_at: "2026", user: { id: 1 } }, { state: "COMMENTED", submitted_at: "2027", user: { id: 1 } }])
add.call("changes_win", reviews: [{ state: "APPROVED", user: { id: 1 } }, { state: "CHANGES_REQUESTED", user: { id: 2 } }])
add.call("same_timestamp_first_wins", reviews: [{ state: "APPROVED", submitted_at: "same", user: { login: "oracle" } }, { state: "CHANGES_REQUESTED", submitted_at: "same", user: { login: "oracle" } }])
add.call("nonarray_reviews", reviews: {})
add.call("review_404", review_status: 404)
add.call("review_bad_json", reviews_raw: "{bad")
add.call("no_ci")
add.call("pending_status", status: { state: "pending", total_count: 2 })
add.call("pending_status_string_count", status: { state: "pending", total_count: "2_0x" })
add.call("passing_runs_ignore_empty_pending", runs: { check_runs: [{ status: "completed", conclusion: "success" }] })
add.call("passing_runs_ignore_real_pending", runs: { check_runs: [{ status: "completed", conclusion: "success" }] }, status: { state: "pending", total_count: 2 })
%w[failure error timed_out action_required success neutral skipped cancelled].each { |conclusion| add.call("run_#{conclusion}", runs: { check_runs: [{ status: "completed", conclusion: }] }) }
add.call("running_check", runs: { check_runs: [{ status: "in_progress", conclusion: "failure" }] })
add.call("combined_failure_wins", runs: { check_runs: [{ status: "completed", conclusion: "success" }] }, status: { state: "error", total_count: 1 })
add.call("checks_404", run_status: 404)
add.call("checks_bad_json", runs_raw: "{bad")
add.call("checks_null_runs", runs: { check_runs: nil })
add.call("mapped_files", mapped: true, files: [{ filename: "a.rs", additions: "2_1x", deletions: 3.9, status: "modified", patch: "fixture-secret-diff" }])
add.call("mapped_file_json", mapped: true, pr: data.merge("title" => "雪<&>\u2028"), files: [{ filename: "雪<&>.rs\u2028", additions: 1, deletions: 0, status: "modified" }])
add.call("unmapped_ignores_files", files: [{ filename: "unused.rs" }])
add.call("files_capped", mapped: true, pr: data.merge("changed_files" => 150), files: (1..105).map { |i| { filename: "#{i}.rs", additions: 1, deletions: 0, status: "added", patch: "fixture-secret-diff" } })
add.call("files_unknown_total", mapped: true, pr: data.merge("changed_files" => 0), files: [{ filename: "a.rs", additions: nil, deletions: nil, status: "modified" }])
add.call("files_nonarray", mapped: true, files: {})
add.call("files_404_preserves", mapped: true, file_status: 404)
add.call("files_bad_json_aborts_update", mapped: true, files_raw: "{bad")
add.call("pr_404", pr_status: 404)
add.call("pr_401", pr_status: 401)
add.call("pr_403", pr_status: 403)
add.call("pr_429", pr_status: 429, headers: { "X-RateLimit-Remaining" => "0", "X-RateLimit-Reset" => "0" })
add.call("pr_500", pr_status: 500)
add.call("pr_bad_json", pr_raw: "{bad")
add.call("pr_missing_head", pr: data.except("head"))
add.call("pr_bad_nested_user", pr: data.merge("user" => []))
add.call("pr_number_title", pr: data.merge("title" => 42, "updated_at" => "bad"))
add.call("no_workspace_token", token: nil)

class FetchFakeHTTP
  def initialize(routes, received) = (@routes, @received = routes, received)
  def get(path, headers)
    @received << { path:, headers: headers.except("Authorization"), authorized: headers.key?("Authorization") }
    test = @routes.fetch(path) { raise "Unexpected GitHub path #{path}" }
    response = Net::HTTPResponse::CODE_TO_OBJ.fetch(test[:status].to_s).new("1.1", test[:status].to_s, "fixture")
    (test[:headers] || {}).each { |key, value| response[key] = value }
    response.instance_variable_set(:@read, true)
    response.body = test[:body]
    response
  end
end
module FetchFakeNetwork
  def start(host, port, **options)
    raise "Unexpected network host #{host}" unless host == "api.github.com" && port == 443 && options[:use_ssl] && options[:open_timeout] == 10 && options[:read_timeout] == 10
    yield Thread.current.fetch(:fetch_http)
  end
end
Net::HTTP.singleton_class.prepend(FetchFakeNetwork)
output = cases.map do |test|
  Github::PullRequestThread.delete_all
  Github::PullRequestThread.create!(pull_request: pr, room:, channel_thread: thread) if test[:mapped]
  pr.update_columns(title: "Previous title", private: true, author_login: "previous", author_avatar_url: nil, state: "closed", base_branch: "old", head_branch: "old", head_sha: "old", html_url: nil, review_decision: "old", check_status: "old", payload: { previous: true }, github_updated_at: nil, fetched_at: 1.hour.ago, fetch_error: "Previous error", changed_files: { files: [{ filename: "old.rs", additions: 1, deletions: 0, status: "added" }], total_count: 1 }.to_json, changed_files_fetched_at: 1.hour.ago, updated_at: 1.hour.ago)
  paths = { pr: "/repos/rails/rails/pulls/123", reviews: "/repos/rails/rails/pulls/123/reviews?per_page=100", runs: "/repos/rails/rails/commits/abc/check-runs?per_page=100", status: "/repos/rails/rails/commits/abc/status", files: "/repos/rails/rails/pulls/123/files?per_page=100" }
  codes = { pr: :pr_status, reviews: :review_status, runs: :run_status, status: :status_status, files: :file_status }
  routes = paths.to_h { |key, path| [path, { status: test.fetch(codes[key], 200), body: test.fetch(:"#{key}_raw") { test[key].to_json }, headers: key == :pr ? test[:headers] : nil }] }
  received = []
  Thread.current[:fetch_http] = FetchFakeHTTP.new(routes, received)
  Github::PullRequestFetcher.new(pr, token: test.fetch(:token, "fixture-workspace-token")).fetch
  pr.reload
  expected = pr.attributes.slice("title", "private", "author_login", "author_avatar_url", "state", "base_branch", "head_branch", "head_sha", "html_url", "review_decision", "check_status", "payload", "fetch_error", "changed_files")
  expected["changed_files"] = JSON.parse(expected["changed_files"]) if expected["changed_files"]
  %w[github_updated_at fetched_at changed_files_fetched_at updated_at].each { |key| expected[key] = pr.public_send(key)&.utc&.strftime("%Y-%m-%d %H:%M:%S") }
  test.merge(expected:, received:, stored_files: pr.changed_files, stored_payload: pr.attributes_before_type_cast["payload"])
end
File.write(ENV.fetch("GITHUB_FETCH_VECTOR_PATH"), JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA"), cases: output }) + "\n")
puts "GitHub fetch Rails oracle: #{output.size} persisted fetch cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
