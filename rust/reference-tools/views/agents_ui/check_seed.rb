# Run through parity/bin/reference runner --seed agents_ui at the frozen clock.
# Mutate a private reference copy and require each new corpus check to reject it.
require "stringio"
validator = File.expand_path("../../campfire/verify_parity_seed.rb", __dir__)
agent = Agent.joins(:user).find_by!(users: { name: "Deploy Bot" })
mutations = {
  ui_owner: -> { agent.update!(owner: nil) },
  ui_credentials: -> { agent.agent_credentials.first.destroy! },
  ui_digest_identifiers: -> { agent.agent_credentials.first.update!(token_last_four: "wrong") },
  ui_credential_states: -> { agent.agent_credentials.update_all(revoked_at: nil) },
  ui_grants: -> { agent.agent_grants.active.first.revoke! },
  ui_revoked_grant: -> { agent.agent_grants.where.not(revoked_at: nil).delete_all },
  ui_steps: -> { AgentStep.where(agent:).first.destroy! },
  ui_pending_and_denied: -> { agent.agent_approvals.where(status: "denied").update_all(status: "cancelled") },
  ui_overdue: -> { agent.agent_approvals.where(status: "pending").update_all(expires_at: 1.day.from_now) },
  ui_ledger: -> { agent.agent_events.update_all(webhook_status: "none") },
  ui_status: -> { agent.update!(status: "idle") }
}
ARGV.replace(["agents_ui"])
mutations.each do |check, mutation|
  ActiveRecord::Base.transaction(requires_new: true) do
    agent.reload
    mutation.call
    stdout, stderr = $stdout, $stderr
    output = StringIO.new
    begin
      $stdout = output
      $stderr = StringIO.new
      load validator
      raise "#{check}: validator accepted the broken corpus"
    rescue SystemExit => error
      raise "#{check}: validator exited successfully" if error.success?
      result = JSON.parse(output.string)
      raise "#{check}: expected check did not fail" unless result.fetch("checks").fetch(check.to_s) == false
    ensure
      $stdout, $stderr = stdout, stderr
    end
    raise ActiveRecord::Rollback
  end
end
puts "Agent UI seed discrimination: #{mutations.size} mutations rejected; transactions restored"
