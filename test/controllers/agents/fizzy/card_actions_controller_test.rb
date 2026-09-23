require "test_helper"

class Agents::Fizzy::CardActionsControllerTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "a bad credential is 401" do
    bad_secret = "wrong-secret"
    post "/agents/fizzy/card_actions",
      params: { kind: "comment", account_id: "897362094", number: 579, body: "hi" }.to_json,
      headers: { "Authorization" => "Bearer #{bad_secret}", "Content-Type" => "application/json" }

    assert_response :unauthorized
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a legacy bot key is 403" do
    post "/agents/fizzy/card_actions?bot_key=#{bot_key_for(users(:bender))}",
      params: { kind: "comment", account_id: "897362094", number: 579, body: "hi" }.to_json,
      headers: { "Content-Type" => "application/json" }

    assert_response :forbidden
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a human session is 403" do
    sign_in :david

    post "/agents/fizzy/card_actions",
      params: { kind: "comment", account_id: "897362094", number: 579, body: "hi" }.to_json,
      headers: { "Content-Type" => "application/json" }

    assert_response :forbidden
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "an agent without workspace-wide external_action is 403" do
    link_owner_fizzy!
    AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "external_action")

    post "/agents/fizzy/card_actions",
      params: { kind: "comment", account_id: "897362094", number: 579, body: "hi" }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "an owner without a linked account is 422" do
    grant_external!

    post "/agents/fizzy/card_actions",
      params: { kind: "comment", account_id: "897362094", number: 579, body: "hi" }.to_json,
      headers: bearer_headers

    assert_response :unprocessable_entity
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a comment request creates one approval and returns 202 without calling Fizzy" do
    account = link_owner_fizzy!
    grant_external!

    assert_difference -> { AgentApproval.count }, 1 do
      post "/agents/fizzy/card_actions",
        params: { kind: "comment", account_id: "897362094", number: 579, body: "Nice work" }.to_json,
        headers: bearer_headers
    end

    assert_response :accepted
    assert_not_requested :any, %r{app\.fizzy\.do}

    approval = AgentApproval.last
    assert_equal "fizzy.comment", approval.action
    assert_equal approval.id, response.parsed_body["id"]
    assert_equal "pending", response.parsed_body["status"]
    assert_equal account.id, approval.fizzy_connected_account_id
    assert_equal "03user1", approval.fizzy_user_id
    assert_equal "David", approval.fizzy_user_name
    assert_nil approval.room_id
  end

  test "a repeated external_id returns the existing row with 200" do
    link_owner_fizzy!
    grant_external!
    params = { kind: "close", account_id: "897362094", number: 579, external_id: "fizzy-1" }

    post "/agents/fizzy/card_actions", params: params.to_json, headers: bearer_headers
    assert_response :accepted
    first_id = response.parsed_body["id"]

    assert_no_difference -> { AgentApproval.count } do
      post "/agents/fizzy/card_actions", params: params.to_json, headers: bearer_headers
    end
    assert_response :ok
    assert_equal first_id, response.parsed_body["id"]
  end

  test "invalid input is 422 with field errors" do
    link_owner_fizzy!
    grant_external!

    post "/agents/fizzy/card_actions",
      params: { kind: "comment", account_id: "897362094", number: 579 }.to_json,
      headers: bearer_headers

    assert_response :unprocessable_entity
    assert response.parsed_body["errors"]["body"].any?
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "fizzy.* approvals cannot be created through POST /agents/approvals" do
    link_owner_fizzy!
    grant_external!

    post "/agents/approvals",
      params: { approval: { action: "fizzy.comment", summary: "Sneaky" } }.to_json,
      headers: bearer_headers

    assert_response :unprocessable_entity
    assert_includes response.parsed_body["error"], "/agents/fizzy/card_actions"
  end

  test "an approved comment runs and reports completion through events polling" do
    link_owner_fizzy!
    grant_external!
    grant_read!
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)

    post "/agents/fizzy/card_actions",
      params: { kind: "comment", account_id: "897362094", number: 579, body: "Nice work" }.to_json,
      headers: bearer_headers
    approval = AgentApproval.find(response.parsed_body["id"])

    perform_enqueued_jobs only: Fizzy::PerformAgentActionJob do
      approval.decide!(decision: "approved", by: users(:david))
    end

    get "/agents/events", headers: bearer_headers
    completion = response.parsed_body.find { |event| event["event_type"] == "fizzy_action_completed" }

    assert_equal "completed", completion["fizzy_action"]["status"]
    assert_equal "fizzy.comment", completion["fizzy_action"]["action"]
    assert_equal "https://app.fizzy.do/897362094/cards/579/comments/03comment1", completion["fizzy_action"]["url"]
    assert_equal approval.id, completion["fizzy_action"]["approval_id"]
  end

  test "a failed action reports its reason without a url" do
    link_owner_fizzy!
    grant_external!
    grant_read!
    stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 403, body: { error: "no access" }.to_json)

    post "/agents/fizzy/card_actions",
      params: { kind: "comment", account_id: "897362094", number: 579, body: "Nice work" }.to_json,
      headers: bearer_headers
    approval = AgentApproval.find(response.parsed_body["id"])

    perform_enqueued_jobs only: Fizzy::PerformAgentActionJob do
      approval.decide!(decision: "approved", by: users(:david))
    end

    get "/agents/events", headers: bearer_headers
    completion = response.parsed_body.find { |event| event["event_type"] == "fizzy_action_completed" }

    assert_equal "failed", completion["fizzy_action"]["status"]
    assert_includes completion["fizzy_action"]["message"], "no access"
    assert_nil completion["fizzy_action"]["url"]
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def grant_external!
      AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "external_action")
    end

    def grant_read!
      AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "read_messages")
    end

    def link_owner_fizzy!(token: "owner-token-abc")
      link_fizzy!(users(:david), token: token)
    end
end
