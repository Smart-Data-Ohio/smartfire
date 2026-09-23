require "test_helper"

class Agents::BudgetsEndpointsTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "PATCH /agents/me sets working presence" do
    patch agents_me_url, params: { working_presence: "Running tests…" },
      headers: auth_headers, as: :json

    assert_response :success
    assert_equal "Running tests…", response.parsed_body["working_presence"]
    assert_equal "Running tests…", @agent.reload.working_presence_text
  end

  test "PATCH /agents/me clears working presence when blank" do
    @agent.set_working_presence!("Thinking…")

    patch agents_me_url, params: { working_presence: "" },
      headers: auth_headers, as: :json

    assert_response :success
    assert_nil response.parsed_body["working_presence"]
    assert_nil @agent.reload.working_presence
  end

  test "PATCH /agents/me rejects overlong presence with 422" do
    patch agents_me_url,
      params: { working_presence: "x" * (Agent::WORKING_PRESENCE_LIMIT + 1) },
      headers: auth_headers, as: :json

    assert_response :unprocessable_entity
  end

  test "member list shows working presence next to the agent" do
    @agent.set_working_presence!("Running tests…")
    sign_in :david

    get room_members_url(@room, format: :json)

    assert_response :success
    bender = response.parsed_body.fetch("members").find { |member| member["id"] == @bot.id }
    assert_equal "Running tests…", bender.fetch("status")
  end

  test "member list falls back to the status note past the presence TTL" do
    @agent.update!(status_note: "On it")
    @agent.set_working_presence!("Thinking…")
    sign_in :david

    travel (Agent::WORKING_PRESENCE_TTL + 1.minute) do
      get room_members_url(@room, format: :json)
    end

    assert_response :success
    bender = response.parsed_body.fetch("members").find { |member| member["id"] == @bot.id }
    assert_equal "On it", bender.fetch("status")
  end

  test "board post budget denies creation with 429" do
    board = Rooms::Board.create_for({ name: "Budgeted", creator: users(:david) },
      users: [ users(:david), @bot ])
    AgentGrant.create!(agent: @agent, room: board, granted_by: users(:david), capability: "post_messages")
    AgentGrant.create!(agent: @agent, room: board, granted_by: users(:david), capability: "manage_threads")
    @agent.update!(daily_board_post_cap: 1)
    ChannelThread.create_board_post!(room: board, creator: @bot, name: "Spent", work_status: "planned")

    assert_no_difference -> { board.channel_threads.count } do
      post room_agent_posts_url(board),
        params: { title: "Over" }.to_json, headers: bearer_headers
    end

    assert_response :too_many_requests
    assert_equal "Daily board post budget exceeded (1/day)", response.parsed_body["error"]
  end

  test "external action budget denies approval requests with 429" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "external_action")
    @agent.update!(daily_external_action_cap: 1)
    AgentApproval.create!(agent: @agent, action: "deploy", summary: "Spent")

    assert_no_difference "AgentApproval.count" do
      post agents_approvals_url,
        params: { approval: { action: "deploy", summary: "Over" } }.to_json,
        headers: bearer_headers
    end

    assert_response :too_many_requests
    assert_equal "Daily external action budget exceeded (1/day)", response.parsed_body["error"]
  end

  test "grants deny before budgets do" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "read_messages")
    @agent.update!(daily_external_action_cap: 1)
    AgentApproval.create!(agent: @agent, action: "deploy", summary: "Spent")

    post agents_approvals_url,
      params: { approval: { action: "deploy", summary: "Over" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def auth_headers
      { "Authorization" => "Bearer #{@secret}" }
    end
end
