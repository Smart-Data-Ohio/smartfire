require "test_helper"

class Agents::DmsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "owner can be DMed" do
    assert_difference -> { Rooms::Direct.count }, 1 do
      post agents_dms_url,
        params: { user_id: users(:david).id, message: { markdown_source: "Hello owner" } }.to_json,
        headers: bearer_headers
    end

    assert_response :created
    room = Room.find(response.parsed_body.dig("room", "id"))
    assert_equal [ @bot.id, users(:david).id ].sort, room.user_ids.sort
    assert_equal room.id, response.parsed_body.dig("room", "id")
    assert_equal "Hello owner", response.parsed_body.dig("message", "body", "plain_text")
    assert_equal @bot.id, response.parsed_body.dig("message", "creator", "id")
    assert_nil response.parsed_body["thread_id"]

    posted = @agent.agent_events.where(event_type: "posted").last
    assert_equal response.parsed_body.dig("message", "id"), posted.message_id
  end

  test "a human who messaged the agent can be DMed" do
    dm = rooms(:bender_and_kevin)
    dm.messages.create!(creator: users(:kevin), body: "Hey bot", client_message_id: "dm-prior-kevin")

    post agents_dms_url,
      params: { user_id: users(:kevin).id, message: { markdown_source: "Right back at you" } }.to_json,
      headers: bearer_headers

    assert_response :created
    assert_equal dm.id, response.parsed_body.dig("room", "id")
    assert_equal "Right back at you", response.parsed_body.dig("message", "body", "plain_text")
  end

  test "dm_anyone capability allows DMing a stranger" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "dm_anyone")
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    post agents_dms_url,
      params: { user_id: users(:jason).id, message: { markdown_source: "Cold hello" } }.to_json,
      headers: bearer_headers

    assert_response :created
    room = Room.find(response.parsed_body.dig("room", "id"))
    assert_equal [ @bot.id, users(:jason).id ].sort, room.user_ids.sort
  end

  test "dm_anyone alone does not grant posting: post_messages is required too" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "dm_anyone")

    post agents_dms_url,
      params: { user_id: users(:jason).id, message: { markdown_source: "Cold hello" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "an agent with all grants revoked cannot DM its owner" do
    grant = AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")
    grant.revoke!

    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "Hello owner" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "a read-only agent cannot DM its owner" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "read_messages")

    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "Hello owner" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "an existing DM denies when posting there was revoked" do
    grant = AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "One" } }.to_json,
      headers: bearer_headers
    assert_response :created

    grant.revoke!
    AgentGrant.create!(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "post_messages")

    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "Two" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "an existing DM allows when posting there still holds" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "One" } }.to_json,
      headers: bearer_headers
    assert_response :created
    first_room_id = response.parsed_body.dig("room", "id")

    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "Two" } }.to_json,
      headers: bearer_headers

    assert_response :created
    assert_equal first_room_id, response.parsed_body.dig("room", "id")
  end

  test "a room-scoped dm_anyone grant does not allow DMing strangers" do
    AgentGrant.new(agent: @agent, room: rooms(:watercooler), granted_by: users(:david), capability: "dm_anyone")
      .save!(validate: false)
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    post agents_dms_url,
      params: { user_id: users(:jason).id, message: { markdown_source: "Cold hello" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability",
      response.parsed_body["error"]
  end

  test "a revoked dm_anyone grant does not allow DMing strangers" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "dm_anyone").revoke!
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    post agents_dms_url,
      params: { user_id: users(:jason).id, message: { markdown_source: "Cold hello" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability",
      response.parsed_body["error"]
  end

  test "strangers are denied without dm_anyone" do
    post agents_dms_url,
      params: { user_id: users(:jason).id, message: { markdown_source: "Cold hello" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability",
      response.parsed_body["error"]
  end

  test "reuses the existing DM room" do
    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "One" } }.to_json,
      headers: bearer_headers
    first_room_id = response.parsed_body.dig("room", "id")

    assert_no_difference -> { Rooms::Direct.count } do
      post agents_dms_url,
        params: { user_id: users(:david).id, message: { markdown_source: "Two" } }.to_json,
        headers: bearer_headers
    end

    assert_response :created
    assert_equal first_room_id, response.parsed_body.dig("room", "id")
  end

  test "accepts top-level message fields" do
    post agents_dms_url,
      params: { user_id: users(:david).id, markdown_source: "Top-level hello" }.to_json,
      headers: bearer_headers

    assert_response :created
    assert_equal "Top-level hello", response.parsed_body.dig("message", "body", "plain_text")
  end

  test "is 404 for unknown users and reply targets outside the DM" do
    post agents_dms_url,
      params: { user_id: 999_999, message: { markdown_source: "Ghost" } }.to_json,
      headers: bearer_headers
    assert_response :not_found

    foreign = rooms(:watercooler).messages.create!(
      creator: users(:david), body: "Elsewhere", client_message_id: "dm-foreign-reply"
    )
    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "Reply", reply_to_message_id: foreign.id } }.to_json,
      headers: bearer_headers
    assert_response :not_found
  end

  test "rejects bots and inactive accounts" do
    other_bot = User.create_bot!(name: "DM Target Bot")

    post agents_dms_url,
      params: { user_id: other_bot.id, message: { markdown_source: "Bot hello" } }.to_json,
      headers: bearer_headers
    assert_response :unprocessable_entity
    assert_equal "Cannot open a DM with a bot", response.parsed_body["error"]

    users(:jason).update!(status: :deactivated)
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "dm_anyone")

    post agents_dms_url,
      params: { user_id: users(:jason).id, message: { markdown_source: "Inactive hello" } }.to_json,
      headers: bearer_headers
    assert_response :unprocessable_entity
    assert_equal "Cannot open a DM with an inactive account", response.parsed_body["error"]
  end

  test "rejects bad credentials" do
    post agents_dms_url,
      params: { user_id: users(:david).id, message: { markdown_source: "Locked" } }.to_json,
      headers: { "Authorization" => "Bearer wrong-secret", "Content-Type" => "application/json" }

    assert_response :unauthorized
  end

  test "creating a DM room is recorded with the agent as actor" do
    assert_difference -> { AuditLog.where(action: "room.create").count }, +1 do
      post agents_dms_url,
        params: { user_id: users(:david).id, message: { markdown_source: "Hello owner" } }.to_json,
        headers: bearer_headers
    end

    assert_response :created
    room = Room.find(response.parsed_body.dig("room", "id"))
    entry = AuditLog.where(action: "room.create").last
    assert_equal @agent.id, entry.actor_id
    assert_equal "Agent Bender Bot", entry.actor_label
    assert_equal room.id, entry.target_id
  end

  test "posting into an existing DM writes no room row" do
    dm = rooms(:bender_and_kevin)
    dm.messages.create!(creator: users(:kevin), body: "Hey bot", client_message_id: "dm-audit-prior-kevin")

    assert_no_difference -> { AuditLog.where(action: "room.create").count } do
      post agents_dms_url,
        params: { user_id: users(:kevin).id, message: { markdown_source: "Right back at you" } }.to_json,
        headers: bearer_headers
    end

    assert_response :created
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end
end
