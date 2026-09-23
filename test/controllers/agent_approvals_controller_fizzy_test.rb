require "test_helper"

class AgentApprovalsControllerFizzyTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @agent = agents(:bender_agent)
  end

  test "an owner without admin rights cannot approve the agent's Fizzy write action" do
    @agent.update!(owner: users(:kevin))
    link_fizzy!(users(:kevin), token: "kevin-token")
    approval = AgentApproval.create!(agent: @agent, action: "fizzy.comment",
      summary: "Comment on Fizzy card #579 (account 897362094): hi", **fizzy_identity)
    sign_in users(:kevin)

    assert_no_enqueued_jobs only: Fizzy::PerformAgentActionJob do
      patch agent_approval_url(approval, decision: "approved"), as: :json
    end

    assert_response :forbidden
    assert_equal "Only an administrator can approve Fizzy write actions", response.parsed_body["error"]
    assert_equal "pending", approval.reload.status
  end

  test "an owner without admin rights may still deny the agent's Fizzy write action" do
    @agent.update!(owner: users(:kevin))
    link_fizzy!(users(:kevin), token: "kevin-token")
    approval = AgentApproval.create!(agent: @agent, action: "fizzy.comment",
      summary: "Comment on Fizzy card #579 (account 897362094): hi", **fizzy_identity)
    sign_in users(:kevin)

    patch agent_approval_url(approval, decision: "denied"), as: :json

    assert_response :success
    assert_equal "denied", approval.reload.status
  end

  test "an administrator approves a Fizzy write action and enqueues execution" do
    approval = AgentApproval.create!(agent: @agent, action: "fizzy.comment",
      summary: "Comment on Fizzy card #579 (account 897362094): hi", **fizzy_identity)
    sign_in :david

    assert_enqueued_with(job: Fizzy::PerformAgentActionJob, args: [ approval.id ]) do
      patch agent_approval_url(approval, decision: "approved"), as: :json
    end

    assert_response :success
    assert_equal "approved", approval.reload.status
  end

  test "approving after the owner's Fizzy account changed is refused" do
    account = link_fizzy!(users(:david), token: "owner-token")
    approval = AgentApproval.create!(agent: @agent, action: "fizzy.comment",
      summary: "Comment on Fizzy card #579 (account 897362094): hi",
      fizzy_connected_account_id: account.id, fizzy_user_id: account.fizzy_user_id, fizzy_user_name: account.fizzy_user_name)
    account.update!(fizzy_user_id: "03someoneelse")
    sign_in :david

    patch agent_approval_url(approval, decision: "approved"), as: :json

    assert_response :unprocessable_entity
    assert_includes response.parsed_body["error"], "Fizzy account changed"
    assert_equal "pending", approval.reload.status
  end

  test "the approval card names the Fizzy identity" do
    approval = AgentApproval.create!(agent: @agent, action: "fizzy.comment",
      summary: "Comment on Fizzy card #579 (account 897362094): hi", **fizzy_identity)
    sign_in :david

    get agent_approvals_url(@agent)

    assert_response :success
    assert_includes response.body, "Acts on Fizzy as David"
  end

  private
    def fizzy_identity
      account = users(:david).fizzy_connected_account || link_fizzy!(users(:david), token: "owner-token")
      {
        fizzy_connected_account_id: account.id,
        fizzy_user_id: account.fizzy_user_id,
        fizzy_user_name: account.fizzy_user_name
      }
    end
end
