require "test_helper"

class AgentKillSwitchTest < ActiveSupport::TestCase
  setup do
    @agent = agents(:bender_agent)
    @owner = @agent.owner
  end

  test "kill switch suspends, cancels pending approvals, and audits" do
    pending = AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")
    expired = AgentApproval.create!(agent: @agent, action: "old", summary: "Stale")
    expired.update_columns(expires_at: 1.hour.ago)
    decided = AgentApproval.create!(agent: @agent, action: "done", summary: "Over")
    decided.update_columns(status: "approved", decided_by_id: @owner.id, decided_at: Time.current)
    @agent.set_working_presence!("Thinking…")

    cancelled = nil
    assert_difference "AuditLog.where(action: %w[ agent.suspend agent.kill_switch ]).count", 2 do
      cancelled = @agent.kill_switch!
    end

    assert_equal 1, cancelled
    assert_predicate @agent.reload, :suspended?
    assert_equal "cancelled", pending.reload.status
    assert_equal "expired", expired.reload.status
    assert_equal "approved", decided.reload.status
    assert_nil @agent.working_presence
    assert_predicate @agent.agent_grants.active, :empty?

    row = AuditLog.where(action: "agent.kill_switch").sole
    assert_equal 1, row.details["pending_approvals_cancelled"]
  end

  test "kill switch marks cancelled approvals' inbox items handled" do
    approval = AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")
    item = ActivityItem.where(source: approval, user: @owner).sole

    @agent.kill_switch!

    assert_predicate item.reload, :handled?
  end

  test "kill switch blocks approved-but-unexecuted external actions" do
    approval = AgentApproval.create!(agent: @agent, action: "github.comment", summary: "Comment",
      room: rooms(:watercooler))
    approval.update_columns(status: "approved", decided_by_id: @owner.id, decided_at: Time.current)

    @agent.kill_switch!

    Github::PerformAgentActionJob.perform_now(approval.id)

    event = @agent.agent_events.where(event_type: "github_action_completed").sole
    assert_equal "failed", event.metadata["status"]
    assert_equal "Agent is suspended or deactivated", event.metadata["message"]
  end

  test "kill switch is safe to repeat" do
    @agent.kill_switch!

    assert_nothing_raised do
      @agent.kill_switch!
    end
  end
end
