require "test_helper"
require "minitest/mock"

class AuditLog::FizzyExecutionAuditTest < ActiveJob::TestCase
  include FizzyTestHelper

  OWNER_TOKEN = "owner-token-abc"

  setup do
    @agent = agents(:bender_agent)
    @grant = AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "external_action")
    @account = link_fizzy!(users(:david), token: OWNER_TOKEN)
  end

  test "a completed Fizzy action is recorded with the decider as actor" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))

    assert_difference -> { AuditLog.where(action: "agent.fizzy_action.execute").count }, +1 do
      Fizzy::PerformAgentActionJob.perform_now(approval.id)
    end

    entry = AuditLog.where(action: "agent.fizzy_action.execute").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal approval.id, entry.target_id
    assert_equal "AgentApproval", entry.target_type
    assert_equal "fizzy.comment", entry.details["action"]
    assert_equal "completed", entry.details["status"]
    assert_equal "https://app.fizzy.do/897362094/cards/579/comments/03comment1", entry.details["url"]
  end

  test "a refused execution is recorded as failed without a Fizzy request" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    @account.update!(fizzy_user_id: "03someoneelse", access_token: "swapped-token")

    assert_difference -> { AuditLog.where(action: "agent.fizzy_action.execute").count }, +1 do
      Fizzy::PerformAgentActionJob.perform_now(approval.id)
    end

    assert_not_requested :any, %r{app\.fizzy\.do}
    entry = AuditLog.where(action: "agent.fizzy_action.execute").last
    assert_equal "failed", entry.details["status"]
    assert_equal "The agent owner's Fizzy account changed since this was approved", entry.details["message"]
  end

  test "a retried job records no second row" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_no_difference -> { AuditLog.where(action: "agent.fizzy_action.execute").count } do
      Fizzy::PerformAgentActionJob.perform_now(approval.id)
    end
  end

  test "a failing audit write still enqueues the outcome webhook" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))

    AuditLog.stub :record!, proc { raise "audit store down" } do
      assert_enqueued_with(job: Agent::EventWebhookJob) do
        Fizzy::PerformAgentActionJob.perform_now(approval.id)
      end
    end

    event = @agent.agent_events.where(event_type: "fizzy_action_completed").last
    assert_equal "completed", event.metadata["status"]
  end

  test "a failing sweep audit write still enqueues the outcome webhook" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    event = @agent.agent_events.create!(
      event_type: "fizzy_action_completed",
      outcome: "delivered",
      agent_approval_id: approval.id,
      webhook_status: "none",
      metadata: { "approval_id" => approval.id, "action" => approval.action, "status" => "running" }
    )
    event.update_columns(created_at: 20.minutes.ago)

    AuditLog.stub :record!, proc { raise "audit store down" } do
      assert_enqueued_with(job: Agent::EventWebhookJob) do
        Fizzy::PerformAgentActionJob.recover_stuck_claims!
      end
    end

    assert_equal "failed", event.reload.metadata["status"]
  end

  test "a stuck claim recovered by the sweep is recorded as failed" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    event = @agent.agent_events.create!(
      event_type: "fizzy_action_completed",
      outcome: "delivered",
      agent_approval_id: approval.id,
      webhook_status: "none",
      metadata: { "approval_id" => approval.id, "action" => approval.action, "status" => "running" }
    )
    event.update_columns(created_at: 20.minutes.ago)

    assert_difference -> { AuditLog.where(action: "agent.fizzy_action.execute").count }, +1 do
      Fizzy::PerformAgentActionJob.recover_stuck_claims!
    end

    entry = AuditLog.where(action: "agent.fizzy_action.execute").last
    assert_equal approval.id, entry.target_id
    assert_equal "failed", entry.details["status"]
    assert_equal "Fizzy action execution timed out", entry.details["message"]
  end

  private
    def build_approval(kind:, number: nil, board_id: nil, title: nil, body: nil, column_id: nil)
      action = Fizzy::AgentCardAction.new(
        account_id: "897362094", kind: kind, board_id: board_id,
        number: number, column_id: column_id, title: title, body: body
      )
      assert action.valid?, action.errors.full_messages.to_sentence
      AgentApproval.create!(
        agent: @agent, action: action.action_name,
        summary: action.summary, payload: action.payload_json,
        fizzy_connected_account_id: @account.id,
        fizzy_user_id: @account.fizzy_user_id,
        fizzy_user_name: @account.fizzy_user_name
      )
    end

    def approve!(approval)
      approval.decide!(decision: "approved", by: users(:david))
      approval
    end
end
