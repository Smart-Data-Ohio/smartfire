require "test_helper"

class Agents::Fizzy::ActionDeliveryTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
    AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "read_messages")
    AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "external_action")
    @account = link_fizzy!(users(:david), token: "owner-token-abc")
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
  end

  test "completion posts the webhook with agent and fizzy_action keys" do
    stub = WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = build_approval(kind: "comment", number: 579, body: "Nice")
    approval.decide!(decision: "approved", by: users(:david))
    perform_enqueued_jobs only: Fizzy::PerformAgentActionJob
    perform_enqueued_jobs only: Agent::EventWebhookJob

    assert_requested stub, times: 2 # the approval decision plus the completion
    completion = @agent.agent_events.where(event_type: "fizzy_action_completed").last
    assert_requested :post, webhooks(:bender).url, body: hash_including(
      "agent" => hash_including("id" => @agent.id, "name" => "Bender Bot", "delivery_id" => completion.id),
      "fizzy_action" => hash_including(
        "approval_id" => approval.id,
        "action" => "fizzy.comment",
        "status" => "completed",
        "url" => "https://app.fizzy.do/897362094/cards/579/comments/03comment1"
      )
    ), times: 1
  end

  test "ack works on completion rows" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = build_approval(kind: "comment", number: 579, body: "Nice")
    approval.decide!(decision: "approved", by: users(:david))
    perform_enqueued_jobs only: Fizzy::PerformAgentActionJob
    event = @agent.agent_events.where(event_type: "fizzy_action_completed").last

    post ack_agents_event_url(event), headers: bearer_headers

    assert_response :success
    assert_equal "acknowledged", response.parsed_body["outcome"]
    assert_equal "acknowledged", event.reload.outcome
  end

  test "the ledger page lists the completion with its status" do
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    approval = build_approval(kind: "comment", number: 579, body: "Nice")
    approval.decide!(decision: "approved", by: users(:david))
    perform_enqueued_jobs only: Fizzy::PerformAgentActionJob

    sign_in :david
    get agent_events_url(@agent)

    assert_response :success
    assert_match "fizzy_action_completed", response.body
    assert_match "Fizzy fizzy.comment: completed", response.body
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

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
end
