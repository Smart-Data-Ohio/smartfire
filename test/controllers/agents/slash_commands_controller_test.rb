require "test_helper"

class Agents::SlashCommandsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "registers a command with post_messages" do
    grant!(capability: "post_messages", room: @room)

    assert_difference -> { AgentSlashCommand.count }, 1 do
      post "/rooms/#{@room.id}/agents/slash_commands",
        params: { name: "deploy", description: "Ship it" }.to_json, headers: bearer_headers
    end

    assert_response :created
    assert_equal(
      { "name" => "deploy", "description" => "Ship it", "room_id" => @room.id, "agent_id" => @agent.id },
      response.parsed_body
    )
  end

  test "a legacy agent without any grants can register" do
    post "/rooms/#{@room.id}/agents/slash_commands",
      params: { name: "deploy" }.to_json, headers: bearer_headers

    assert_response :created
  end

  test "re-registering the agent's own name updates it" do
    grant!(capability: "post_messages", room: @room)
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy", description: "Old")

    assert_no_difference -> { AgentSlashCommand.count } do
      post "/rooms/#{@room.id}/agents/slash_commands",
        params: { name: "deploy", description: "New" }.to_json, headers: bearer_headers
    end

    assert_response :created
    assert_equal "New", AgentSlashCommand.sole.description
  end

  test "requires post_messages in the room" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    post "/rooms/#{@room.id}/agents/slash_commands",
      params: { name: "deploy" }.to_json, headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "rejects built-in names" do
    grant!(capability: "post_messages", room: @room)

    post "/rooms/#{@room.id}/agents/slash_commands",
      params: { name: "poll" }.to_json, headers: bearer_headers

    assert_response :unprocessable_entity
    assert_match "built-in", response.parsed_body["error"]
  end

  test "rejects names held by another agent" do
    grant!(capability: "post_messages", room: @room)
    other = Agent.create!(user: User.create_bot!(name: "Other Bot"), owner: users(:david), kind: :workspace)
    AgentSlashCommand.create!(agent: other, room: @room, name: "deploy")

    post "/rooms/#{@room.id}/agents/slash_commands",
      params: { name: "deploy" }.to_json, headers: bearer_headers

    assert_response :unprocessable_entity
    assert_match "already registered", response.parsed_body["error"]
  end

  test "a registration lost to another agent's concurrent create reports taken" do
    grant!(capability: "post_messages", room: @room)
    other = Agent.create!(user: User.create_bot!(name: "Other Bot"), owner: users(:david), kind: :workspace)
    winner = AgentSlashCommand.create!(agent: other, room: @room, name: "deploy")
    # The lookup missed the row a concurrent request was inserting; the
    # unique index rejects the save instead.
    AgentSlashCommand.stubs(:find_by).returns(nil, winner)
    AgentSlashCommand.any_instance.stubs(:save!).raises(ActiveRecord::RecordNotUnique)

    post "/rooms/#{@room.id}/agents/slash_commands",
      params: { name: "deploy" }.to_json, headers: bearer_headers

    assert_response :unprocessable_entity
    assert_match "already registered", response.parsed_body["error"]
  end

  test "a re-registration lost to its own concurrent create still succeeds" do
    grant!(capability: "post_messages", room: @room)
    winner = AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy", description: "Old")
    AgentSlashCommand.stubs(:find_by).returns(nil, winner)
    AgentSlashCommand.any_instance.stubs(:save!).raises(ActiveRecord::RecordNotUnique)

    post "/rooms/#{@room.id}/agents/slash_commands",
      params: { name: "deploy", description: "New" }.to_json, headers: bearer_headers

    assert_response :created
    assert_equal "New", winner.reload.description
  end

  test "is 404 for rooms the agent cannot see" do
    grant!(capability: "post_messages")

    post "/rooms/#{rooms(:designers).id}/agents/slash_commands",
      params: { name: "deploy" }.to_json, headers: bearer_headers

    assert_response :not_found
  end

  test "unregisters the agent's own command" do
    grant!(capability: "post_messages", room: @room)
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")

    assert_difference -> { AgentSlashCommand.count }, -1 do
      delete "/rooms/#{@room.id}/agents/slash_commands/deploy", headers: bearer_headers
    end

    assert_response :success
    assert_equal true, response.parsed_body["unregistered"]
  end

  test "unregistering requires post_messages too" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")

    delete "/rooms/#{@room.id}/agents/slash_commands/deploy", headers: bearer_headers

    assert_response :forbidden
    assert AgentSlashCommand.exists?(room: @room, name: "deploy")
  end

  test "unregistering another agent's command is 404" do
    grant!(capability: "post_messages", room: @room)
    other = Agent.create!(user: User.create_bot!(name: "Other Bot"), owner: users(:david), kind: :workspace)
    AgentSlashCommand.create!(agent: other, room: @room, name: "deploy")

    delete "/rooms/#{@room.id}/agents/slash_commands/deploy", headers: bearer_headers

    assert_response :not_found
    assert AgentSlashCommand.exists?(room: @room, name: "deploy")
  end

  test "rejects session requests" do
    sign_in :david

    post "/rooms/#{@room.id}/agents/slash_commands",
      params: { name: "deploy" }.to_json, headers: { "Content-Type" => "application/json" }

    assert_response :forbidden
  end

  private
    def grant!(capability:, room: nil)
      AgentGrant.create!(agent: @agent, room: room, granted_by: users(:david), capability: capability)
    end

    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end
end
