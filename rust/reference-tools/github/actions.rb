# Pinned agent execution and action validation oracle. Fixed-host HTTP is entirely local.
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
Account.create!(name: "Action oracle")
bot = User.create!(id: 810, name: "Machine", role: :bot)
owner = User.create!(id: 811, name: "Oracle", email_address: "oracle@example.test", password: "fixture-password", role: :administrator)
agent = Agent.create!(id: 812, user: bot, owner:)
room = Rooms::Closed.create!(id: 815, name: "Action room", creator: owner)
Membership.create!(user: bot, room:)
message = room.messages.create!(id: 818, creator: owner, body: "Discussion")
thread = ChannelThread.create!(id: 817, room:, creator: owner, parent_message: message)
pr = Github::PullRequest.create!(id: 816, owner: "rails", repo: "rails", number: 12)
Github::PullRequestThread.create!(pull_request: pr, room:, channel_thread: thread)
grant = AgentGrant.create!(id: 820, agent:, room:, granted_by: owner, capability: "external_action")
account = GithubConnectedAccount.create!(id: 819, user: bot, github_login: "machine", access_token: "fixture-agent-token")
validation = [
  {kind: "comment", body: "  "}, {kind: "request_changes"}, {kind: "approve"},
  {kind: "request_review", reviewers: "  "}, {kind: "request_review", reviewers: "alice,bob!!"},
  {kind: "request_review", reviewers: (1..16).map { |i| "user#{i}" }},
  {kind: "request_review", reviewers: " @Alice, alice  @BOB "}, {kind: "merge", body: "x"},
  {kind: "comment", body: "x"*3501}, {kind: "comment", body: "雪"*3500},
  {kind: "comment", body: "  Nice  ", reviewers: "alice"}, {kind: "comment", body: "雪"*200},
  {kind: "request_changes", body: "Fix it"}, {kind: "approve", body: " optional "},
  {kind: "request_review", reviewers: ["@Alice, Bob", nil]},
  {kind: "request_review", reviewers: ["a-b", "a--b"]},
  {kind: "request_review", reviewers: "a\u00a0b"}, {kind: "request_review", reviewers: "@@alice"},
  {kind: "request_review", reviewers: (1..15).map { |i| "u#{i}" }},
  {kind: "request_review", reviewers: ["a"*39, "a"*40]},
  {kind: "comment", body: "\u0000\tNice\v\u0000"}, {kind: "comment", body: 42},
  {kind: "comment", body: false}, {kind: "comment", body: ["one", "two"]},
  {kind: "request_review", reviewers: "ſalice"}, {kind: "request_review", reviewers: "ß"},
  {kind: "request_review", reviewers: "K"}, {kind: "request_review", reviewers: "ſ"*39},
  {kind: "request_review", reviewers: "ſ"*40},
  {kind: nil}, {kind: 1}, {kind: "approve", body: "x"*3501}
].map do |input|
  action = Github::AgentPullRequestAction.new(pull_request: pr, **input)
  valid = action.valid?
  input.merge(expected: {valid:, errors: action.errors.to_hash, body: action.normalized_body, reviewers: action.normalized_reviewers, action: action.action_name, summary: action.summary, payload: action.payload_hash, payload_json: action.payload_json})
end
class ActionFakeHTTP
  def initialize(test, received) = (@test, @received = test, received)
  def post(path, body, headers)
    expected_token = @test[:owner_app] ? "fixture-owner-token" : "fixture-agent-token"
    @received << { method: "POST", path:, body: JSON.parse(body), agent_authorization: headers["Authorization"] == "Bearer #{expected_token}" }
    GithubConnectedAccount.find(819).delete if @test[:destroy_during_http]
    code = @test.fetch(:code, 201)
    response = Net::HTTPResponse::CODE_TO_OBJ.fetch(code.to_s).new("1.1", code.to_s, "fixture")
    response.instance_variable_set(:@read, true)
    response.body = @test.fetch(:response) { {html_url: "https://github.com/rails/rails/pull/12#fixture"} }.to_json
    response
  end
end
module ActionFakeNetwork
  def start(host, port, **options)
    raise "Unexpected network host #{host}" unless host == "api.github.com" && port == 443 && options[:use_ssl] && options[:open_timeout] == 10 && options[:read_timeout] == 10
    yield Thread.current.fetch(:action_http)
  end
end
Net::HTTP.singleton_class.prepend(ActionFakeNetwork)
module ActionAuditFailure
  def record!(**args)
    raise "fixture audit down" if Thread.current[:action_audit_failure]
    super
  end
end
AuditLog.singleton_class.prepend(ActionAuditFailure)
cases = [
  {name: "comment", body: "  Nice  "}, {name: "approve", kind: "approve"},
  {name: "request_changes", kind: "request_changes", body: " Fix it "},
  {name: "request_review", kind: "request_review", reviewers: " @Alice, bob, ALICE "},
  {name: "approve_without_body", kind: "approve", body: nil},
  {name: "optional_approve_body", kind: "approve", body: " Looks good "},
  {name: "denied", status: "denied"}, {name: "pending_expired", status: "pending", expired: true},
  {name: "cancelled", status: "cancelled"}, {name: "suspended", suspended: true},
  {name: "deactivated", deactivated: true}, {name: "missing_room", missing_room: true},
  {name: "soft_deleted_room", soft_deleted: true}, {name: "membership_removed", membership_removed: true},
  {name: "grant_revoked", grant_revoked: true}, {name: "legacy_without_grants", legacy: true},
  {name: "workspace_grant", workspace_grant: true}, {name: "mapping_removed", mapping_removed: true},
  {name: "no_payload", payload: nil}, {name: "invalid_json", payload: "{bad"},
  {name: "array_payload", payload: "[]"}, {name: "missing_pr", missing_pr: true},
  {name: "invalid_body", body: " "}, {name: "invalid_reviewers", kind: "request_review", reviewers: "alice!"},
  {name: "summary_swapped", summary_swapped: true}, {name: "action_swapped", action_swapped: true},
  {name: "pr_swapped", pr_swapped: true}, {name: "account_relinked", relinked: true},
  {name: "account_replaced", replaced: true}, {name: "missing_identity", missing_identity: true},
  {name: "login_case", login_case: true}, {name: "disconnected", disconnected: true},
  {name: "destroyed", destroyed: true}, {name: "unreadable_access", unreadable: true},
  {name: "unauthorized", code: 401, response: {message: "Bad credentials"}},
  {name: "destroyed_during_401", destroy_during_http: true, code: 401, response: {message: "Bad credentials"}},
  {name: "refused", code: 403, response: {message: "Nope"}},
  {name: "server_error", code: 500, response: {message: "Nope"}},
  {name: "non_hash_response", response: []}, {name: "historical", historical: true},
  {name: "running_claim", running: true}, {name: "twice", twice: true},
  {name: "failure_twice", grant_revoked: true, twice: true},
  {name: "missing_approval", missing_approval: true}, {name: "non_github", non_github: true},
  {name: "owner_app", owner_app: true}, {name: "owner_pat", owner_pat: true},
  {name: "webhook", webhook: true}, {name: "audit_failure", webhook: true, audit_failure: true}
]
output = cases.map do |test|
  result = nil
  begin
    AgentEvent.delete_all
    AgentApproval.delete_all
    Webhook.delete_all
    GithubConnectedAccount.delete_all
    agent.reload.update_columns(suspended_at: nil)
    bot.reload.update_columns(status: :active)
    room.reload.update_columns(deleted_at: nil)
    Membership.where(user: bot, room:).delete_all
    Membership.create!(user: bot, room:)
    AgentGrant.delete_all
    grant = AgentGrant.create!(id: 820, agent:, room:, granted_by: owner, capability: "external_action")
    Github::PullRequestThread.delete_all
    Github::PullRequestThread.create!(pull_request: pr, room:, channel_thread: thread)
    Github::PullRequest.where(id: 822).delete_all
    account = GithubConnectedAccount.create!(id: 819, user: bot, github_login: "machine", access_token: "fixture-agent-token")
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    AuditLog.delete_all
    action = Github::AgentPullRequestAction.new(pull_request: pr, kind: test.fetch(:kind, "comment"), body: test.fetch(:body, "Nice"), reviewers: test[:reviewers])
    payload = action.payload_hash
    payload["pull_request_id"] = 999 if test[:missing_pr]
    if test[:pr_swapped]
      other = Github::PullRequest.create!(id: 822, owner: "other", repo: "repo", number: 13)
      Github::PullRequestThread.where(pull_request: pr).update_all(github_pull_request_id: other.id)
      payload["pull_request_id"] = other.id
    end
    linked = account
    if test[:owner_app] || test[:owner_pat]
      owner_account = GithubConnectedAccount.create!(id: 821, user: owner, github_login: "owner", access_token: "fixture-owner-token", token_source: test[:owner_app] ? "app" : "pat", refresh_token: "fixture-refresh", token_expires_at: 1.hour.from_now)
      linked = owner_account if test[:owner_app]
    end
    approval = AgentApproval.create!(id: 813, agent:, room:, action: test[:non_github] ? "deploy" : test[:action_swapped] ? "github.approve" : action.action_name, summary: test[:summary_swapped] ? "Swapped" : action.summary, payload: test.key?(:payload) ? test[:payload] : payload.to_json, github_account_id: test[:missing_identity] ? nil : linked.id, github_login: test[:missing_identity] ? nil : test[:login_case] ? linked.github_login.upcase : linked.github_login)
    approval.update_columns(status: test.fetch(:status, "approved"), decided_by_id: owner.id)
    approval.update_columns(expires_at: 1.minute.ago) if test[:expired]
    agent.update_columns(suspended_at: Time.current) if test[:suspended]
    bot.update_columns(status: :deactivated) if test[:deactivated]
    approval.update_columns(room_id: nil) if test[:missing_room]
    room.update_columns(deleted_at: Time.current) if test[:soft_deleted]
    Membership.where(user: bot, room:).delete_all if test[:membership_removed]
    grant.update_columns(revoked_at: Time.current) if test[:grant_revoked]
    AgentGrant.delete_all if test[:legacy]
    grant.update_columns(room_id: nil) if test[:workspace_grant]
    Github::PullRequestThread.delete_all if test[:mapping_removed]
    account.update_columns(github_login: "someone-else") if test[:relinked]
    if test[:replaced]
      account.destroy!
      GithubConnectedAccount.create!(id: 823, user: bot, github_login: "machine", access_token: "fixture-replaced-token")
    end
    account.update_columns(disconnected_reason: "Disconnected fixture") if test[:disconnected]
    account.destroy! if test[:destroyed]
    ActiveRecord::Base.connection.execute("UPDATE github_connected_accounts SET access_token='bogus-ciphertext' WHERE id=819") if test[:unreadable]
    bot.create_webhook!(url: "https://example.com/hooks") if test[:webhook]
    if test[:historical] || test[:running]
      AgentEvent.create!(id: 814, agent:, room:, agent_approval_id: test[:historical] ? nil : approval.id, event_type: "github_action_completed", outcome: "delivered", metadata: {approval_id: approval.id, action: approval.action, status: test[:running] ? "running" : "completed"})
    end
    approval.delete if test[:missing_approval]
    received = []
    Thread.current[:action_http] = ActionFakeHTTP.new(test, received)
    Thread.current[:action_audit_failure] = test[:audit_failure]
    Github::PerformAgentActionJob.perform_now(approval.id)
    Github::PerformAgentActionJob.perform_now(approval.id) if test[:twice]
    Thread.current[:action_audit_failure] = false
    events = AgentEvent.order(:id).map { |e| e.attributes.slice("agent_id", "agent_approval_id", "room_id", "actor_id", "outcome", "detail", "event_type", "metadata", "webhook_status", "webhook_attempts", "hop") }
    audits = AuditLog.order(:id).map { |a| a.attributes.slice("action", "actor_id", "actor_label", "target_type", "target_id", "target_label", "details") }
    jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j| {class: j[:job].name, attempt: j[:args][1]} }
    result = test.merge(approval: approval.attributes.slice("action", "summary", "payload", "status", "github_account_id", "github_login", "room_id"), expected: {events:, audits:, jobs:, received:, disconnected_reason: GithubConnectedAccount.find_by(id: 819)&.disconnected_reason})
  end
  result
end
File.write(ENV.fetch("GITHUB_ACTION_VECTOR_PATH"), JSON.pretty_generate({reference_pin: ENV.fetch("PARITY_REFERENCE_SHA"), validation:, cases: output}) + "\n")
puts "GitHub agent Rails oracle: #{output.size} execution cases, #{validation.size} action validation cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
