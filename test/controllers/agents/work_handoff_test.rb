require "test_helper"

class Agents::WorkHandoffTest < ActionDispatch::IntegrationTest
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:bender) ])
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
    @receiver_bot = User.create_bot!(name: "Handoff Receiver")
    @receiver = @receiver_bot.create_agent!(kind: :workspace, owner: users(:david))
    @board.memberships.grant_to(@receiver_bot)
    _, @receiver_secret = AgentCredential.create_with_secret!(agent: @receiver, name: "receiver", created_by: users(:david))
    grant!(@agent, "read_messages")
    grant!(@agent, "post_messages")
    grant!(@agent, "manage_threads")
    grant!(@receiver, "read_messages")
    grant!(@receiver, "post_messages")
    grant!(@receiver, "manage_threads")
    @thread = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Ship it", work_status: "in_progress", owner_id: @bot.id)
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
  end

  test "an agent hands off its thread with a context package" do
    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "Halfway there",
        links: [ "https://example.com/spec" ], open_questions: [ "Which API?" ] }.to_json,
      headers: bearer_headers(@secret)

    assert_response :created
    assert_equal @receiver_bot.id, response.parsed_body.dig("owner", "id")
    assert_equal @receiver_bot.id, @thread.reload.work_owner_id
    assert_equal "Halfway there", response.parsed_body.dig("handoff", "summary")
    assert_equal [ "https://example.com/spec" ], response.parsed_body.dig("handoff", "links")
    assert_equal [ "Which API?" ], response.parsed_body.dig("handoff", "open_questions")

    event = @thread.work_thread_events.ordered.first
    assert_equal "work_handoff", event.event_type
    assert_equal @bot.id, event.actor_id

    assert_equal 1, AuditLog.where(action: "work.handoff", target_id: @thread.id).count
  end

  test "the receiver polls the handoff with its context package" do
    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "Halfway",
        links: [ "https://example.com/a" ], open_questions: [ "Why?" ] }.to_json,
      headers: bearer_headers(@secret)
    assert_response :created

    get agents_events_url, headers: bearer_headers(@receiver_secret)

    assert_response :success
    handed_off = response.parsed_body.find { |row| row["event_type"] == "work_handed_off" }
    assert handed_off, "expected a work_handed_off row, got: #{response.parsed_body.map { |row| row["event_type"] }}"
    assert_equal @thread.id, handed_off.dig("work", "id")
    assert_equal "Halfway", handed_off.dig("handoff", "summary")
    assert_equal [ "https://example.com/a" ], handed_off.dig("handoff", "links")
    assert_equal [ "Why?" ], handed_off.dig("handoff", "open_questions")
    assert_equal "Bender Bot", handed_off.dig("handoff", "sender_name")
  end

  test "the receiver acks the handoff row" do
    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "Halfway" }.to_json,
      headers: bearer_headers(@secret)

    event = @receiver.agent_events.deliverable.order(:id).last
    post ack_agents_event_url(event), headers: bearer_headers(@receiver_secret)

    assert_response :success
    assert_equal "acknowledged", event.reload.outcome
  end

  test "the handoff enqueues the receiver webhook with the package" do
    @receiver_bot.create_webhook!(url: "https://receiver.example.com/hook")
    stub = WebMock.stub_request(:post, "https://receiver.example.com/hook").to_return(status: 200)

    perform_enqueued_jobs do
      post agents_work_thread_url(@thread) + "/handoff",
        params: { receiver_agent_id: @receiver.id, summary: "Halfway" }.to_json,
        headers: bearer_headers(@secret)
      assert_response :created
    end

    assert_requested stub
    event = @receiver.agent_events.order(:id).last
    assert_equal "delivered", event.reload.webhook_status
  end

  test "a thread the agent does not own is 404" do
    other = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Mine", work_status: "planned", owner_id: users(:david).id)

    post agents_work_thread_url(other) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "Nope" }.to_json,
      headers: bearer_headers(@secret)

    assert_response :not_found
  end

  test "a missing manage_threads grant is 403" do
    AgentGrant.where(agent: @agent, capability: "manage_threads").update_all(revoked_at: Time.current)

    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "Nope" }.to_json,
      headers: bearer_headers(@secret)

    assert_response :forbidden
    assert_equal "Forbidden: agent lacks manage_threads capability", response.parsed_body["error"]
  end

  test "a human session is 403" do
    sign_in :david

    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "Nope" }.to_json,
      headers: { "Content-Type" => "application/json" }

    assert_response :forbidden
  end

  test "an ineligible receiver is 422" do
    outsider = User.create_bot!(name: "Outsider").create_agent!(kind: :workspace, owner: users(:david))

    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: outsider.id, summary: "Nope" }.to_json,
      headers: bearer_headers(@secret)

    assert_response :unprocessable_entity
    assert_match "active agent member", response.parsed_body["error"]
  end

  test "a receiver missing manage_threads is 422" do
    AgentGrant.where(agent: @receiver, capability: "manage_threads").update_all(revoked_at: Time.current)

    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "Nope" }.to_json,
      headers: bearer_headers(@secret)

    assert_response :unprocessable_entity
    assert_match "manage_threads", response.parsed_body["error"]
  end

  test "handing off to the current owner is 422" do
    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @agent.id, summary: "Again" }.to_json,
      headers: bearer_headers(@secret)

    assert_response :unprocessable_entity
    assert_match "already the owner", response.parsed_body["error"]
  end

  test "an over-capped package is 422 and keeps the owner" do
    post agents_work_thread_url(@thread) + "/handoff",
      params: { receiver_agent_id: @receiver.id, summary: "x" * (WorkHandoff::SUMMARY_LIMIT + 1) }.to_json,
      headers: bearer_headers(@secret)

    assert_response :unprocessable_entity
    assert_equal @bot.id, @thread.reload.work_owner_id
  end

  test "handoffs throttle at 60 a minute per credential" do
    with_memory_cache do
      freeze_time do
        60.times do |index|
          thread = ChannelThread.create_board_post!(room: @board, creator: users(:david),
            name: "Work #{index}", work_status: "planned", owner_id: @bot.id)
          post agents_work_thread_url(thread) + "/handoff",
            params: { receiver_agent_id: @receiver.id, summary: "Yours" }.to_json,
            headers: bearer_headers(@secret)
          assert_response :created
          thread.update_work!(actor: users(:david), work_owner_id: @bot.id)
        end

        post agents_work_thread_url(@thread) + "/handoff",
          params: { receiver_agent_id: @receiver.id, summary: "One too many" }.to_json,
          headers: bearer_headers(@secret)

        assert_response :too_many_requests
        assert_equal "rate_limited", response.parsed_body["error"]
        assert response.headers["Retry-After"].present?
      end
    end
  end

  private
    def grant!(agent, capability)
      AgentGrant.create!(agent: agent, room: @board, granted_by: users(:david), capability: capability)
    end

    def bearer_headers(secret)
      { "Authorization" => "Bearer #{secret}", "Content-Type" => "application/json" }
    end

    def with_memory_cache
      store = ActiveSupport::Cache::MemoryStore.new
      previous = Rails.cache
      Rails.cache = store
      yield store
    ensure
      Rails.cache = previous
    end
end
