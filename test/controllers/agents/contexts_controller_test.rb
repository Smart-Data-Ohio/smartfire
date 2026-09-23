require "test_helper"

class Agents::ContextsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "returns the trigger, window, authors, and room for a room message" do
    first = @room.messages.create!(creator: users(:david), body: "First", client_message_id: "ctx-room-1")
    second = @room.messages.create!(creator: @bot, body: "Second", client_message_id: "ctx-room-2")
    trigger = @room.messages.create!(creator: users(:kevin), body: "Third", client_message_id: "ctx-room-3")
    after = @room.messages.create!(creator: users(:david), body: "After trigger", client_message_id: "ctx-room-4")

    get agents_context_url(message_id: trigger.id), headers: bearer_headers

    assert_response :success
    body = response.parsed_body
    assert_equal trigger.id, body.dig("message", "id")
    assert_nil body["thread"]
    assert_nil body["root_message"]
    ids = body["messages"].map { |message| message["id"] }
    assert_equal [ first.id, second.id, trigger.id ], ids.last(3)
    assert_not_includes ids, after.id
    assert_equal ids.sort, ids
    assert_equal @room.id, body.dig("room", "id")
    assert_equal @room.name, body.dig("room", "name")

    by_name = body["authors"].index_by { |author| author["name"] }
    assert_equal({ "agent" => false, "human" => true }, by_name["David"].slice("agent", "human"))
    assert_equal({ "agent" => true, "human" => false }, by_name["Bender Bot"].slice("agent", "human"))

    bot_message = body["messages"].find { |message| message.dig("creator", "name") == "Bender Bot" }
    assert_equal true, bot_message.dig("creator", "agent")
    assert_equal false, bot_message.dig("creator", "human")
  end

  test "returns the thread summary and root message for a thread reply" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Context thread")
    ThreadMembership.join!(thread, users(:david))
    root = @room.messages.create!(creator: users(:david), body: "Root", client_message_id: "ctx-thread-root")
    thread.update!(parent_message: root)
    reply = thread.post_message!(creator: @bot, attributes: { body: "Reply" })

    get agents_context_url(message_id: reply.id), headers: bearer_headers

    assert_response :success
    body = response.parsed_body
    assert_equal reply.id, body.dig("message", "id")
    assert_equal thread.id, body.dig("thread", "id")
    assert_equal "Context thread", body.dig("thread", "name")
    assert_equal root.id, body.dig("root_message", "id")
    assert_equal [ reply.id ], body["messages"].map { |message| message["id"] }
  end

  test "thread_id alone returns the thread tail with a null trigger" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Tail thread")
    ThreadMembership.join!(thread, users(:david))
    first = thread.post_message!(creator: users(:david), attributes: { body: "One" })
    second = thread.post_message!(creator: @bot, attributes: { body: "Two" })

    get agents_context_url(thread_id: thread.id), headers: bearer_headers

    assert_response :success
    body = response.parsed_body
    assert_nil body["message"]
    assert_equal thread.id, body.dig("thread", "id")
    assert_equal [ first.id, second.id ], body["messages"].map { |message| message["id"] }
  end

  test "message and thread must agree" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Other thread")
    ThreadMembership.join!(thread, users(:david))
    trigger = @room.messages.create!(creator: users(:david), body: "Root trigger", client_message_id: "ctx-mismatch")

    get agents_context_url(message_id: trigger.id, thread_id: thread.id), headers: bearer_headers

    assert_response :unprocessable_entity
    assert_equal "Message is not in the given thread", response.parsed_body["error"]
  end

  test "requires message_id or thread_id" do
    get agents_context_url, headers: bearer_headers

    assert_response :unprocessable_entity
    assert_equal "message_id or thread_id is required", response.parsed_body["error"]
  end

  test "defaults to 30 messages and caps at 100" do
    trigger = nil
    105.times do |index|
      trigger = @room.messages.create!(creator: users(:david), body: "Bulk #{index}", client_message_id: "ctx-bulk-#{index}")
    end

    get agents_context_url(message_id: trigger.id), headers: bearer_headers
    assert_response :success
    assert_equal 30, response.parsed_body["messages"].size
    assert_equal trigger.id, response.parsed_body["messages"].last["id"]

    get agents_context_url(message_id: trigger.id, limit: 500), headers: bearer_headers
    assert_response :success
    assert_equal 100, response.parsed_body["messages"].size

    get agents_context_url(message_id: trigger.id, limit: 5), headers: bearer_headers
    assert_response :success
    assert_equal 5, response.parsed_body["messages"].size
    assert_equal trigger.id, response.parsed_body["messages"].last["id"]
  end

  test "is 404 for unknown messages and threads" do
    get agents_context_url(message_id: 999_999), headers: bearer_headers
    assert_response :not_found

    get agents_context_url(thread_id: 999_999), headers: bearer_headers
    assert_response :not_found
  end

  test "is 404 for rooms the agent is not a member of" do
    foreign = rooms(:designers).messages.create!(
      creator: users(:david), body: "Stranger", client_message_id: "ctx-foreign"
    )

    get agents_context_url(message_id: foreign.id), headers: bearer_headers

    assert_response :not_found
  end

  test "a mismatched thread_id in a foreign room is still 404" do
    foreign = rooms(:designers).messages.create!(
      creator: users(:david), body: "Stranger", client_message_id: "ctx-foreign-mismatch"
    )
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Decoy thread")
    ThreadMembership.join!(thread, users(:david))

    get agents_context_url(message_id: foreign.id, thread_id: thread.id), headers: bearer_headers

    assert_response :not_found
  end

  test "a mismatched thread_id without a read grant is still 403" do
    AgentGrant.create!(agent: @agent, room: rooms(:bender_and_kevin), granted_by: users(:david), capability: "read_messages")
    trigger = @room.messages.create!(creator: users(:david), body: "Unreadable", client_message_id: "ctx-unreadable-mismatch")
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Decoy thread")
    ThreadMembership.join!(thread, users(:david))

    get agents_context_url(message_id: trigger.id, thread_id: thread.id), headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks read_messages capability", response.parsed_body["error"]
  end

  test "requires read_messages" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "post_messages")
    trigger = @room.messages.create!(creator: users(:david), body: "Gated", client_message_id: "ctx-gated")

    get agents_context_url(message_id: trigger.id), headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks read_messages capability", response.parsed_body["error"]
  end

  test "denies member rooms without a read grant with 403" do
    AgentGrant.create!(agent: @agent, room: rooms(:bender_and_kevin), granted_by: users(:david), capability: "read_messages")
    trigger = @room.messages.create!(creator: users(:david), body: "Unreadable", client_message_id: "ctx-unreadable")

    get agents_context_url(message_id: trigger.id), headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks read_messages capability", response.parsed_body["error"]
  end

  test "rejects bad credentials and session requests" do
    trigger = @room.messages.create!(creator: users(:david), body: "Locked", client_message_id: "ctx-locked")

    get agents_context_url(message_id: trigger.id),
      headers: { "Authorization" => "Bearer wrong-secret", "Content-Type" => "application/json" }
    assert_response :unauthorized

    sign_in :david
    get agents_context_url(message_id: trigger.id)
    assert_response :forbidden
    assert_equal "Forbidden: Bearer #{""}agent token required", response.parsed_body["error"]
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end
end
