require "test_helper"

class Fizzy::PerformAgentActionJobTest < ActiveJob::TestCase
  include FizzyTestHelper

  OWNER_TOKEN = "owner-token-abc"

  setup do
    @agent = agents(:bender_agent)
    @grant = AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "external_action")
    @account = link_fizzy!(users(:david), token: OWNER_TOKEN)
  end

  test "approving a fizzy action enqueues the job, denying does not" do
    approvable = build_approval(kind: "comment", number: 579, body: "Nice")

    assert_enqueued_with(job: Fizzy::PerformAgentActionJob, args: [ approvable.id ]) do
      approvable.decide!(decision: "approved", by: users(:david))
    end

    deniable = build_approval(kind: "comment", number: 579, body: "Nope")
    assert_no_enqueued_jobs only: Fizzy::PerformAgentActionJob do
      deniable.decide!(decision: "denied", by: users(:david))
    end
  end

  test "a comment completes with the Fizzy url" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .with(headers: { "Authorization" => "Bearer #{OWNER_TOKEN}" })
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    event = completion_event_for(approval)
    assert_equal "completed", event.metadata["status"]
    assert_equal "fizzy.comment", event.metadata["action"]
    assert_equal "https://app.fizzy.do/897362094/cards/579/comments/03comment1", event.metadata["url"]
    assert_nil event.message_id
    assert_nil event.room_id
  end

  test "a create completes with the new card url" do
    stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 201, body: fizzy_card_payload(number: 580).to_json)
    approval = approve!(build_approval(kind: "create", board_id: "03board1", title: "Ship it"))

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    event = completion_event_for(approval)
    assert_equal "completed", event.metadata["status"]
    assert_equal "https://app.fizzy.do/897362094/cards/580", event.metadata["url"]
  end

  test "a revoked grant fails without a request" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    @grant.revoke!

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_failed_with(approval, "Agent no longer has the external_action capability")
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a suspended agent fails without a request" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    @agent.update!(suspended_at: Time.current)

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_failed_with(approval, "Agent is suspended or deactivated")
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a relinked owner account after approval makes no request" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    @account.update!(fizzy_user_id: "03someoneelse", access_token: "swapped-token")

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_failed_with(approval, "The agent owner's Fizzy account changed since this was approved")
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a disconnected owner account fails without a request" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    @account.mark_disconnected!("Fizzy rejected the linked token (401)")

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_failed_with(approval, "Agent owner has no usable Fizzy account")
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a payload that no longer matches the summary fails without a request" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    approval.update_columns(payload: { "account_id" => "897362094", "kind" => "close", "number" => 579 }.to_json)

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_failed_with(approval, "Approval summary does not match its payload")
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a read-only owner token fails without disconnecting" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 401, body: {}.to_json)
    stub_fizzy_identity(OWNER_TOKEN)
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_failed_with(approval, "The agent owner's Fizzy token is read-only; card writes need a Read + Write token")
    assert_predicate @account.reload, :connected?
  end

  test "a revoked owner token fails and disconnects" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 401, body: {}.to_json)
    stub_request(:get, "https://app.fizzy.do/my/identity.json").to_return(status: 401, body: {}.to_json)
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))

    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_failed_with(approval, "Fizzy rejected the agent owner's linked token (401)")
    assert_not @account.reload.connected?
  end

  test "a second run finds the earlier outcome and makes no request" do
    stub = stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))

    Fizzy::PerformAgentActionJob.perform_now(approval.id)
    Fizzy::PerformAgentActionJob.perform_now(approval.id)

    assert_requested stub, times: 1
    assert_equal 1, @agent.agent_events.where(event_type: "fizzy_action_completed").count
  end

  test "an unknown approval id is a no-op" do
    Fizzy::PerformAgentActionJob.perform_now(123_456_789)

    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "recover_stuck_claims! fails running claims past the window" do
    approval = approve!(build_approval(kind: "comment", number: 579, body: "Nice work"))
    event = @agent.agent_events.create!(
      event_type: "fizzy_action_completed",
      outcome: "delivered",
      agent_approval_id: approval.id,
      webhook_status: "none",
      metadata: { "approval_id" => approval.id, "action" => approval.action, "status" => "running" }
    )
    event.update_columns(created_at: 20.minutes.ago)

    Fizzy::PerformAgentActionJob.recover_stuck_claims!

    assert_equal "failed", event.reload.metadata["status"]
    assert_equal "Fizzy action execution timed out", event.metadata["message"]
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

    def completion_event_for(approval)
      @agent.agent_events.where(event_type: "fizzy_action_completed", outcome: "delivered").to_a
        .find { |event| event.metadata.is_a?(Hash) && event.metadata["approval_id"] == approval.id } ||
        raise(ActiveRecord::RecordNotFound, "missing fizzy_action_completed event for approval #{approval.id}")
    end

    def assert_failed_with(approval, reason)
      event = completion_event_for(approval)
      assert_equal "failed", event.metadata["status"]
      assert_equal reason, event.metadata["message"]
      assert_equal "fizzy.comment", event.metadata["action"]
      assert_nil event.metadata["url"]
      assert_equal "delivered", event.outcome
    end
end
