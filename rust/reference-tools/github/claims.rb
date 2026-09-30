# Persisted outcome oracle for both integration sweepers. No outbound HTTP.
require "json"
require "digest"
require "net/http"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
module NoClaimNetwork
  def start(*) = raise("Unexpected outbound HTTP during claim recovery")
end
Net::HTTP.singleton_class.prepend(NoClaimNetwork)
ActiveJob::Base.queue_adapter = :test
ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose = false
load Rails.root.join("db/schema.rb")
travel_to Time.utc(2026, 1, 1, 12)
Account.create!(name: "Claim oracle")
bot = User.create!(id: 810, name: "Machine", role: :bot)
owner = User.create!(id: 811, name: "Oracle", email_address: "oracle@example.test", password: "fixture-password", role: :administrator)
agent = Agent.create!(id: 812, user: bot, owner:)
approval = AgentApproval.create!(id: 813, agent:, action: "github.comment", summary: "Approved fixture", expires_at: 24.hours.from_now)
approval.update_columns(status: "approved", decided_by_id: owner.id)
cases = [
  { name: "fresh", age: 899 }, { name: "boundary", age: 900 }, { name: "overdue", age: 901 },
  { name: "blank_url_webhook", age: 960, webhook: true },
  { name: "historical_approval", age: 960, historical: true },
  { name: "missing_approval", age: 960, missing: true },
  { name: "late_finish", age: 960, webhook: true, late_finish: true },
  { name: "finish_before_sweep", age: 960, webhook: true, finish_first: true },
  { name: "fizzy", age: 960, fizzy: true },
  { name: "audit_failure", age: 960, webhook: true, audit_failure: true }
]
module ClaimAuditFailure
  def record!(**args)
    raise "fixture audit down" if Thread.current[:claim_audit_failure]
    super
  end
end
AuditLog.singleton_class.prepend(ClaimAuditFailure)
output = cases.map do |test|
  AgentEvent.delete_all
  AuditLog.delete_all
  Webhook.delete_all
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  action = test[:fizzy] ? "fizzy.comment" : "github.comment"
  approval.update_columns(action:)
  bot.create_webhook!(url: "https://example.com/hooks") if test[:webhook]
  bot.webhook&.update_columns(url: "")
  bot.reload # clear its association cache between cases
  event = AgentEvent.create!(id: 814, agent:, actor: owner, event_type: test[:fizzy] ? "fizzy_action_completed" : "github_action_completed", outcome: "delivered", agent_approval_id: test[:historical] ? nil : test[:missing] ? 999 : approval.id, webhook_attempts: 2, created_at: test[:age].seconds.ago, metadata: { approval_id: test[:missing] ? 999 : approval.id, action:, status: "running", extra: "preserved" })
  job = test[:fizzy] ? Fizzy::PerformAgentActionJob : Github::PerformAgentActionJob
  finish = -> {
    job.new.send(:finish_claim, event, approval, agent, status: "completed", url: "https://github.com/rails/rails/pull/12#issuecomment-9")
  }
  finish.call if test[:finish_first]
  Thread.current[:claim_audit_failure] = test[:audit_failure]
  job.recover_stuck_claims!(now: Time.current)
  Thread.current[:claim_audit_failure] = false
  finish.call if test[:late_finish]
  event.reload
  expected = { metadata: event.metadata, detail: event.detail, webhook_status: event.webhook_status, webhook_next_attempt_at: event.webhook_next_attempt_at&.utc&.strftime("%Y-%m-%d %H:%M:%S"), webhook_attempts: event.webhook_attempts, outcome: event.outcome, created_at: event.created_at.utc.strftime("%Y-%m-%d %H:%M:%S"), audits: AuditLog.count, jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.size }
  audits = AuditLog.order(:id).map { |a| a.attributes.slice("action", "actor_id", "actor_label", "target_type", "target_id", "target_label", "details") }
  jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j| { class: j[:job].name, args: j[:args] } }
  test.merge(expected:, audits:, jobs:)
end
File.write(ENV.fetch("GITHUB_CLAIM_VECTOR_PATH"), JSON.pretty_generate({ reference_pin: "d7c7de92", cases: output }) + "\n")
puts "GitHub/Fizzy claim Rails oracle: #{output.size} persisted outcome cases; reference d7c7de92"
