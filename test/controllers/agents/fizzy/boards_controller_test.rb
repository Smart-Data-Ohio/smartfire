require "test_helper"

class Agents::Fizzy::BoardsControllerTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "a bad credential is 401" do
    bad_secret = "wrong-secret"
    get "/agents/fizzy/boards",
      headers: { "Authorization" => "Bearer #{bad_secret}" }

    assert_response :unauthorized
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a legacy bot key is 403" do
    get "/agents/fizzy/boards?bot_key=#{bot_key_for(users(:bender))}"

    assert_response :forbidden
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a human session is 403" do
    sign_in :david

    get "/agents/fizzy/boards"

    assert_response :forbidden
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "an agent without the fizzy capability is 403" do
    link_owner_fizzy!

    get "/agents/fizzy/boards", headers: bearer_headers

    assert_response :forbidden
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a room-scoped fizzy grant is 403: reads need it workspace-wide" do
    link_owner_fizzy!
    AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "fizzy")

    get "/agents/fizzy/boards", headers: bearer_headers

    assert_response :forbidden
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "index lists the owner's boards with the workspace-wide grant" do
    account = link_owner_fizzy!
    grant_fizzy!
    stub = stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .with(headers: { "Authorization" => "Bearer #{account.access_token}" })
      .to_return(status: 200, body: [ fizzy_board_payload ].to_json)

    get "/agents/fizzy/boards", headers: bearer_headers

    assert_response :success
    assert_requested stub
    assert_equal "Engineering", response.parsed_body.first["name"]
  end

  test "show returns the board with its columns" do
    link_owner_fizzy!
    grant_fizzy!
    stub_request(:get, "https://app.fizzy.do/897362094/boards/03board1.json")
      .to_return(status: 200, body: fizzy_board_payload.to_json)
    stub_request(:get, "https://app.fizzy.do/897362094/boards/03board1/columns.json")
      .to_return(status: 200, body: [ fizzy_column_payload ].to_json)

    get "/agents/fizzy/boards/03board1", headers: bearer_headers

    assert_response :success
    assert_equal "Engineering", response.parsed_body["board"]["name"]
    assert_equal "In Progress", response.parsed_body["columns"].first["name"]
  end

  test "an unknown board is 404" do
    link_owner_fizzy!
    grant_fizzy!
    stub_request(:get, "https://app.fizzy.do/897362094/boards/03missing.json")
      .to_return(status: 404, body: {}.to_json)

    get "/agents/fizzy/boards/03missing", headers: bearer_headers

    assert_response :not_found
  end

  test "an invalid board id is 404 without a request" do
    link_owner_fizzy!
    grant_fizzy!

    get "/agents/fizzy/boards/03board!x", headers: bearer_headers

    assert_response :not_found
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "an owner without a linked account is 422" do
    grant_fizzy!

    get "/agents/fizzy/boards", headers: bearer_headers

    assert_response :unprocessable_entity
    assert_equal "Agent owner has no usable Fizzy account", response.parsed_body["error"]
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a rejected owner token disconnects the account" do
    account = link_owner_fizzy!
    grant_fizzy!
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .to_return(status: 401, body: {}.to_json)

    get "/agents/fizzy/boards", headers: bearer_headers

    assert_response :unprocessable_entity
    assert_not account.reload.connected?
  end

  test "a Fizzy outage is a 502" do
    link_owner_fizzy!
    grant_fizzy!
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json").to_return(status: 500, body: "boom")

    get "/agents/fizzy/boards", headers: bearer_headers

    assert_response :bad_gateway
  end

  test "a suspended agent is 401" do
    link_owner_fizzy!
    grant_fizzy!
    @agent.update!(suspended_at: Time.current)

    get "/agents/fizzy/boards", headers: bearer_headers

    assert_response :unauthorized
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def grant_fizzy!
      AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "fizzy")
    end

    def link_owner_fizzy!(token: "owner-token-abc")
      link_fizzy!(users(:david), token: token)
    end
end
