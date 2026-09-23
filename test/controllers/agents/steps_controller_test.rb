require "test_helper"

class Agents::StepsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
    @message = @room.root_messages.create!(creator: @bot,
      markdown_source: "Working on it", client_message_id: "steps-own")
  end

  test "creates a step on the agent's own message" do
    assert_difference "AgentStep.count", 1 do
      post agents_steps_url,
        params: { message_id: @message.id, name: "Run tests", status: "running",
          input_summary: "bundle exec rails test", duration_ms: 1200 }.to_json,
        headers: bearer_headers
    end

    assert_response :created
    assert_equal "Run tests", response.parsed_body["name"]
    assert_equal @message.id, response.parsed_body["message_id"]
    assert_nil response.parsed_body["thread_id"]
  end

  test "creates a step on an owned work thread" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: @bot)
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "manage_threads")

    assert_difference "AgentStep.count", 1 do
      post agents_steps_url,
        params: { thread_id: thread.id, name: "Reproduce", output_summary: "Found it" }.to_json,
        headers: bearer_headers
    end

    assert_response :created
    assert_equal thread.id, response.parsed_body["thread_id"]
  end

  test "updates a step" do
    step = AgentStep.create!(agent: @agent, message: @message, name: "Run tests")

    patch agents_step_url(step),
      params: { status: "done", output_summary: "Green", duration_ms: 1500 }.to_json,
      headers: bearer_headers

    assert_response :success
    assert_equal "done", response.parsed_body["status"]
    assert_equal "Run tests", step.reload.name
    assert_equal "Green", step.output_summary
  end

  test "requires exactly one parent" do
    post agents_steps_url,
      params: { name: "Nowhere" }.to_json, headers: bearer_headers
    assert_response :unprocessable_entity

    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: @bot)
    post agents_steps_url,
      params: { message_id: @message.id, thread_id: thread.id, name: "Both" }.to_json,
      headers: bearer_headers
    assert_response :unprocessable_entity
  end

  test "denies steps on another author's message with 404" do
    foreign = @room.root_messages.create!(creator: users(:david),
      markdown_source: "Mine", client_message_id: "steps-foreign")

    assert_no_difference "AgentStep.count" do
      post agents_steps_url,
        params: { message_id: foreign.id, name: "Hijack" }.to_json,
        headers: bearer_headers
    end

    assert_response :not_found
  end

  test "denies message steps without post_messages" do
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "read_messages")

    post agents_steps_url,
      params: { message_id: @message.id, name: "Denied" }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks post_messages capability", response.parsed_body["error"]
  end

  test "denies thread steps without manage_threads" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: @bot)
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "post_messages")

    post agents_steps_url,
      params: { thread_id: thread.id, name: "Denied" }.to_json,
      headers: bearer_headers

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks manage_threads capability", response.parsed_body["error"]
  end

  test "denies steps on unowned threads with 404" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Fix it",
      work_status: "in_progress", work_owner: users(:david))
    AgentGrant.create!(agent: @agent, room: @room, granted_by: users(:david), capability: "manage_threads")

    post agents_steps_url,
      params: { thread_id: thread.id, name: "Hijack" }.to_json,
      headers: bearer_headers

    assert_response :not_found
  end

  test "denies updating another agent's step with 404" do
    other = create_agent_in(@room, name: "Other Stepper")
    foreign_message = @room.root_messages.create!(creator: other.user,
      markdown_source: "Theirs", client_message_id: "steps-theirs")
    foreign_step = AgentStep.create!(agent: other, message: foreign_message, name: "Theirs")

    patch agents_step_url(foreign_step),
      params: { status: "done" }.to_json, headers: bearer_headers

    assert_response :not_found
  end

  test "enforces the per-parent step cap" do
    AgentStep::MAX_PER_PARENT.times do |n|
      AgentStep.create!(agent: @agent, message: @message, name: "Step #{n}")
    end

    post agents_steps_url,
      params: { message_id: @message.id, name: "One too many" }.to_json,
      headers: bearer_headers

    assert_response :unprocessable_entity
  end

  test "escapes step content in the rendered message" do
    AgentStep.create!(agent: @agent, message: @message, name: "<b>Bold</b>",
      input_summary: "<script>alert(1)</script>", output_summary: "a & b")

    sign_in :david
    get room_url(@room)

    assert_response :success
    assert_includes response.body, "&lt;b&gt;Bold&lt;/b&gt;"
    assert_includes response.body, "&lt;script&gt;"
    assert_includes response.body, "a &amp; b"
    assert_not_includes response.body, "<script>alert(1)</script>"
  end

  test "rejects session requests" do
    sign_in :david
    post agents_steps_url, params: { message_id: @message.id, name: "Human" }
    assert_response :forbidden
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def create_agent_in(room, name:)
      bot = User.create_bot!(name: name)
      agent = bot.create_agent!(kind: :workspace, owner: users(:david))
      room.memberships.grant_to(bot)
      agent
    end
end
