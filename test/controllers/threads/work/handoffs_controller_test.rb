require "test_helper"

class Threads::Work::HandoffsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz), users(:bender) ])
    @agent = agents(:bender_agent)
    grant!("read_messages")
    grant!("post_messages")
    grant!("manage_threads")
    @thread = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Ship it", work_status: "in_progress", owner_id: users(:david).id)
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
  end

  test "a manager opens the handoff form" do
    sign_in :david

    get new_thread_work_handoff_path(@thread)

    assert_response :success
    assert_match "Hand off", response.body
    assert_match "Bender Bot", response.body
  end

  test "the current owner opens the handoff form" do
    @thread.update_work!(actor: users(:david), work_owner_id: users(:jz).id)
    sign_in :jz

    get new_thread_work_handoff_path(@thread)

    assert_response :success
  end

  test "a plain member is forbidden" do
    sign_in :jz

    get new_thread_work_handoff_path(@thread)
    assert_response :forbidden

    post thread_work_handoff_path(@thread), params: { receiver_agent_id: @agent.id, summary: "Nope" }
    assert_response :forbidden
  end

  test "a non-member gets 404" do
    sign_in :jason

    get new_thread_work_handoff_path(@thread)
    assert_response :not_found

    post thread_work_handoff_path(@thread), params: { receiver_agent_id: @agent.id, summary: "Nope" }
    assert_response :not_found
  end

  test "an untracked thread is 422" do
    plain = ChannelThread.create!(room: rooms(:designers), creator: users(:david), name: "Chat")
    ThreadMembership.join!(plain, users(:david))
    sign_in :david

    post thread_work_handoff_path(plain), params: { receiver_agent_id: @agent.id, summary: "Nope" }

    assert_response :unprocessable_entity
  end

  test "a person hands off with a context package" do
    sign_in :david

    assert_difference -> { WorkHandoff.count }, 1 do
      post thread_work_handoff_path(@thread), params: {
        receiver_agent_id: @agent.id,
        summary: "Halfway there",
        links: "https://example.com/spec",
        open_questions: "Which API?"
      }
    end

    assert_redirected_to room_thread_path(@board, @thread)
    assert_equal users(:bender).id, @thread.reload.work_owner_id

    handoff = WorkHandoff.order(:id).last
    assert_equal users(:david).id, handoff.sender_id
    assert_equal [ "https://example.com/spec" ], handoff.links
    assert_equal [ "Which API?" ], handoff.open_questions

    event = @thread.work_thread_events.ordered.first
    assert_equal "work_handoff", event.event_type

    ledger = @agent.agent_events.deliverable.order(:id).last
    assert_equal "work_handed_off", ledger.event_type

    assert_equal 1, AuditLog.where(action: "work.handoff", target_id: @thread.id).count
  end

  test "a manager hands off a thread owned by someone else" do
    @thread.update_work!(actor: users(:david), work_owner_id: users(:jz).id)
    sign_in :david

    post thread_work_handoff_path(@thread), params: { receiver_agent_id: @agent.id, summary: "To the agent" }

    assert_redirected_to room_thread_path(@board, @thread)
    assert_equal users(:bender).id, @thread.reload.work_owner_id
  end

  test "handoff answers json with the work payload and package" do
    sign_in :david

    post thread_work_handoff_path(@thread, format: :json), params: {
      receiver_agent_id: @agent.id, summary: "Halfway", links: [ "https://example.com/a" ], open_questions: []
    }

    assert_response :created
    assert_equal @thread.id, response.parsed_body["id"]
    assert_equal users(:bender).id, response.parsed_body.dig("owner", "id")
    assert_equal "Halfway", response.parsed_body.dig("handoff", "summary")
    assert_equal [ "https://example.com/a" ], response.parsed_body.dig("handoff", "links")
  end

  test "handoff rejects an agent outside the room" do
    outsider = Agent.create!(user: users(:kevin), owner: users(:david))
    sign_in :david

    assert_no_difference -> { WorkHandoff.count } do
      post thread_work_handoff_path(@thread), params: { receiver_agent_id: outsider.id, summary: "Nope" }
    end

    assert_response :unprocessable_entity
    assert_match "active agent member", response.body
  end

  test "handoff rejects a receiver missing manage_threads" do
    AgentGrant.where(agent: @agent, capability: "manage_threads").update_all(revoked_at: Time.current)
    sign_in :david

    post thread_work_handoff_path(@thread), params: { receiver_agent_id: @agent.id, summary: "Nope" }

    assert_response :unprocessable_entity
    assert_match "manage_threads", response.body
  end

  test "handoff rejects the current owner" do
    @thread.update_work!(actor: users(:david), work_owner_id: users(:bender).id)
    sign_in :david

    post thread_work_handoff_path(@thread), params: { receiver_agent_id: @agent.id, summary: "Again" }

    assert_response :unprocessable_entity
    assert_match "already the owner", response.body
  end

  test "handoff rejects an over-capped package" do
    sign_in :david

    assert_no_difference -> { WorkHandoff.count } do
      post thread_work_handoff_path(@thread), params: {
        receiver_agent_id: @agent.id, summary: "x" * (WorkHandoff::SUMMARY_LIMIT + 1)
      }
    end

    assert_response :unprocessable_entity
    assert_equal users(:david).id, @thread.reload.work_owner_id
  end

  test "the handoff summary renders escaped in work history" do
    sign_in :david
    post thread_work_handoff_path(@thread), params: {
      receiver_agent_id: @agent.id, summary: "<script>alert(1)</script>"
    }

    get room_thread_path(@board, @thread)

    assert_response :success
    assert_match "&lt;script&gt;", response.body
    assert_no_match "<script>alert(1)</script>", response.body
  end

  private
    def grant!(capability)
      AgentGrant.create!(agent: @agent, room: @board, granted_by: users(:david), capability: capability)
    end
end
