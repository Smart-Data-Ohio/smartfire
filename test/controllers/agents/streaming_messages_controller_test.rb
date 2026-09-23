require "test_helper"

class Agents::StreamingMessagesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "starts a streaming root message" do
    assert_difference -> { @room.messages.count }, 1 do
      post room_agent_streaming_messages_url(@room),
        params: { message: { markdown_source: "Hello", client_message_id: "stream-root" } }.to_json,
        headers: bearer_headers
    end

    assert_response :created
    assert_equal true, response.parsed_body["streaming"]
    assert_nil response.parsed_body["thread_id"]

    message = @room.messages.order(:id).last
    assert_predicate message, :streaming?
    assert_equal @bot.id, message.creator_id
    assert_equal "Hello", message.plain_text_body
    assert_empty @agent.agent_events.where(message_id: message.id)
  end

  test "starts an empty stream and appends to it" do
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "", client_message_id: "stream-empty" } }.to_json,
      headers: bearer_headers
    assert_response :created
    id = response.parsed_body["id"]

    patch agents_streaming_message_url(id),
      params: { append: "Hello " }.to_json, headers: bearer_headers
    assert_response :success

    patch agents_streaming_message_url(id),
      params: { append: "there" }.to_json, headers: bearer_headers
    assert_response :success

    assert_equal "Hello there", Message.find(id).plain_text_body
    assert_predicate Message.find(id), :streaming?
  end

  test "replaces the stream body" do
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Draft", client_message_id: "stream-replace" } }.to_json,
      headers: bearer_headers
    id = response.parsed_body["id"]

    patch agents_streaming_message_url(id),
      params: { markdown_source: "Final" }.to_json, headers: bearer_headers
    assert_response :success

    assert_equal "Final", Message.find(id).plain_text_body
  end

  test "whitespace-only appends land instead of 422" do
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Title", client_message_id: "stream-ws" } }.to_json,
      headers: bearer_headers
    id = response.parsed_body["id"]

    patch agents_streaming_message_url(id),
      params: { append: "\n\n" }.to_json, headers: bearer_headers
    assert_response :success

    patch agents_streaming_message_url(id),
      params: { append: " " }.to_json, headers: bearer_headers
    assert_response :success

    assert_equal "Title\n\n ", Message.find(id).markdown_source
  end

  test "update requires append or markdown_source" do
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Draft", client_message_id: "stream-nothing" } }.to_json,
      headers: bearer_headers
    id = response.parsed_body["id"]

    patch agents_streaming_message_url(id), params: {}.to_json, headers: bearer_headers

    assert_response :unprocessable_entity
  end

  test "streams inside a channel thread" do
    thread = create_thread!(room: @room, creator: users(:david))

    post room_agent_streaming_messages_url(@room),
      params: { thread_id: thread.id,
        message: { markdown_source: "On it.", client_message_id: "stream-thread" } }.to_json,
      headers: bearer_headers

    assert_response :created
    assert_equal thread.id, response.parsed_body["thread_id"]
    assert_equal "On it.", thread.messages.sole.plain_text_body
    assert_predicate thread.messages.sole, :streaming?
  end

  test "update and finalize pause while the thread is locked" do
    thread = create_thread!(room: @room, creator: users(:david))

    post room_agent_streaming_messages_url(@room),
      params: { thread_id: thread.id,
        message: { markdown_source: "On it.", client_message_id: "stream-locked" } }.to_json,
      headers: bearer_headers
    id = response.parsed_body["id"]

    thread.lock_conversation!

    patch agents_streaming_message_url(id),
      params: { append: "Denied" }.to_json, headers: bearer_headers
    assert_response :unprocessable_entity
    assert_equal "This thread is locked", response.parsed_body["error"]

    post finalize_agents_streaming_message_url(id), headers: bearer_headers
    assert_response :unprocessable_entity
    assert_equal "This thread is locked", response.parsed_body["error"]
    assert_predicate Message.find(id), :streaming?

    thread.unlock_conversation!

    post finalize_agents_streaming_message_url(id), headers: bearer_headers
    assert_response :success
    assert_not Message.find(id).streaming?

    thread.lock_conversation!

    post finalize_agents_streaming_message_url(id), headers: bearer_headers
    assert_response :success
  end

  test "finalize fires side effects exactly once" do
    watcher = create_agent_in(@room, name: "Finalize Watcher")
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Hey @[David] and @[Finalize Watcher] hovercraft",
        client_message_id: "stream-finalize" } }.to_json,
      headers: bearer_headers
    id = response.parsed_body["id"]

    assert_no_difference [ "ActivityItem.count", "AgentEvent.count" ] do
      patch agents_streaming_message_url(id),
        params: { append: " eels" }.to_json, headers: bearer_headers
      assert_response :success
    end

    assert_difference -> { ActivityItem.where(source_id: id).count }, 1 do
      assert_difference -> { @agent.agent_events.where(event_type: "posted").count }, 1 do
        assert_difference -> { watcher.agent_events.where(event_type: "mention").count }, 1 do
          post finalize_agents_streaming_message_url(id), headers: bearer_headers
          assert_response :success
        end
      end
    end

    assert_equal false, response.parsed_body["streaming"]
    assert_equal [ Message.find(id) ], @room.messages.search("hovercraft")

    assert_no_difference [ "ActivityItem.count", "AgentEvent.count" ] do
      post finalize_agents_streaming_message_url(id), headers: bearer_headers
      assert_response :success
    end
  end

  test "retried stream start returns the original message" do
    payload = { message: { markdown_source: "Once", client_message_id: "stream-retry" } }

    post room_agent_streaming_messages_url(@room), params: payload.to_json, headers: bearer_headers
    assert_response :created
    original_id = response.parsed_body["id"]

    assert_no_difference -> { @room.messages.count } do
      post room_agent_streaming_messages_url(@room), params: payload.to_json, headers: bearer_headers
      assert_response :created
    end

    assert_equal original_id, response.parsed_body["id"]
  end

  test "requires post_messages to start a stream" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Denied" } }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "requires post_messages to update and finalize" do
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Mine", client_message_id: "stream-revoke" } }.to_json,
      headers: bearer_headers
    id = response.parsed_body["id"]
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    patch agents_streaming_message_url(id),
      params: { append: "Denied" }.to_json, headers: bearer_headers
    assert_response :forbidden

    post finalize_agents_streaming_message_url(id), headers: bearer_headers
    assert_response :forbidden
  end

  test "cannot update or finalize another author's message" do
    foreign = @room.root_messages.create!(creator: users(:david),
      markdown_source: "Mine", client_message_id: "stream-foreign")

    patch agents_streaming_message_url(foreign.id),
      params: { append: "Hijack" }.to_json, headers: bearer_headers
    assert_response :not_found

    post finalize_agents_streaming_message_url(foreign.id), headers: bearer_headers
    assert_response :not_found
  end

  test "cannot update a finalized message" do
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Done", client_message_id: "stream-done" } }.to_json,
      headers: bearer_headers
    id = response.parsed_body["id"]
    post finalize_agents_streaming_message_url(id), headers: bearer_headers
    assert_response :success

    patch agents_streaming_message_url(id),
      params: { append: "Too late" }.to_json, headers: bearer_headers

    assert_response :unprocessable_entity
    assert_equal "Message is not streaming", response.parsed_body["error"]
  end

  test "is 404 for rooms the agent is not a member of" do
    AgentGrant.create!(agent: @agent, granted_by: users(:david), capability: "post_messages")

    post room_agent_streaming_messages_url(rooms(:designers)),
      params: { message: { markdown_source: "Intruder" } }.to_json,
      headers: bearer_headers

    assert_response :not_found
  end

  test "rejects session requests" do
    sign_in :david
    post room_agent_streaming_messages_url(@room),
      params: { message: { markdown_source: "Human" } }
    assert_response :forbidden
  end

  test "message budget denies stream starts with 429" do
    @agent.update!(daily_message_cap: 1)
    @room.root_messages.create!(creator: @bot,
      markdown_source: "Spent", client_message_id: "stream-spent")

    assert_no_difference -> { @room.messages.count } do
      post room_agent_streaming_messages_url(@room),
        params: { message: { markdown_source: "Over" } }.to_json,
        headers: bearer_headers
    end

    assert_response :too_many_requests
    assert_equal "Daily message budget exceeded (1/day)", response.parsed_body["error"]
    assert_equal "messages", response.parsed_body["cap"]
  end

  test "message budget denies plain posts with 429 and notifies once" do
    @agent.update!(daily_message_cap: 1)
    @room.root_messages.create!(creator: @bot,
      markdown_source: "Spent", client_message_id: "post-spent")

    assert_no_difference -> { @room.messages.count } do
      post room_agent_messages_url(@room),
        params: { message: { body: "Over", client_message_id: "post-over" } }.to_json,
        headers: bearer_headers
    end
    assert_response :too_many_requests

    post room_agent_messages_url(@room),
      params: { message: { body: "Over again", client_message_id: "post-over-2" } }.to_json,
      headers: bearer_headers
    assert_response :too_many_requests

    assert_equal 1, ActivityItem.where(event_type: "agent_budget_exceeded").count
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def finalize_agents_streaming_message_url(id)
      "/agents/streaming_messages/#{id}/finalize"
    end

    def create_agent_in(room, name:)
      bot = User.create_bot!(name: name)
      agent = bot.create_agent!(kind: :workspace, owner: users(:david))
      room.memberships.grant_to(bot)
      agent
    end

    def create_thread!(room:, creator:)
      thread = ChannelThread.create!(room: room, creator: creator, name: "Stream thread")
      ThreadMembership.join!(thread, creator)
      thread
    end
end
