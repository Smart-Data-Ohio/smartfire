require "test_helper"

class ChannelThreadHandoffTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz), users(:bender) ])
    @agent = agents(:bender_agent)
    grant!(@agent, "read_messages")
    grant!(@agent, "post_messages")
    grant!(@agent, "manage_threads")
    @thread = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Ship it", work_status: "in_progress", owner_id: users(:david).id)
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
  end

  test "hand_off! transfers ownership and records the handoff in work history" do
    handoff = @thread.hand_off!(
      sender: users(:david), receiver_agent: @agent,
      summary: "Halfway there", links: [ "https://example.com/spec" ], open_questions: [ "Which API?" ]
    )

    assert_equal users(:bender).id, @thread.reload.work_owner_id
    assert_equal @thread.id, handoff.channel_thread_id

    event = @thread.work_thread_events.ordered.first
    assert_equal "work_handoff", event.event_type
    assert_equal users(:david).id, event.actor_id
    assert_equal users(:david).id, event.from_owner_id
    assert_equal users(:bender).id, event.to_owner_id
    assert_equal handoff.id, event.metadata["handoff_id"]
    assert_equal "Halfway there", event.metadata["handoff_summary"]
    assert_equal 1, event.metadata["handoff_links_count"]
    assert_equal 1, event.metadata["handoff_questions_count"]
  end

  test "hand_off! writes a work_handed_off ledger row with the context snapshot" do
    @thread.hand_off!(sender: users(:david), receiver_agent: @agent,
      summary: "Halfway", links: [ "https://example.com/a" ], open_questions: [ "Why?" ])

    event = @agent.agent_events.deliverable.order(:id).last
    assert_equal "work_handed_off", event.event_type
    assert_equal "delivered", event.outcome
    assert_equal @board.id, event.room_id
    assert_equal users(:david).id, event.actor_id
    assert_equal @thread.id, event.metadata["thread_id"]
    assert_equal "Halfway", event.metadata.dig("handoff", "summary")
    assert_equal [ "https://example.com/a" ], event.metadata.dig("handoff", "links")
    assert_equal [ "Why?" ], event.metadata.dig("handoff", "open_questions")
  end

  test "hand_off! unassigns a previous agent owner" do
    other_user = User.create!(name: "Clippy", email_address: "clippy@example.com", password: "password123456", role: "bot")
    other = Agent.create!(user: other_user, owner: users(:david), kind: "workspace")
    @board.memberships.grant_to(other_user)
    grant!(other, "read_messages")
    grant!(other, "post_messages")
    @thread.update_work!(actor: users(:david), work_owner_id: other_user.id)

    @thread.hand_off!(sender: users(:david), receiver_agent: @agent, summary: "Yours now")

    assert_equal "work_unassigned", other.agent_events.deliverable.order(:id).last.event_type
    assert_equal "work_handed_off", @agent.agent_events.deliverable.order(:id).last.event_type
  end

  test "hand_off! records work.handoff in the audit log" do
    @thread.hand_off!(sender: users(:david), receiver_agent: @agent, summary: "Yours now")

    row = AuditLog.where(action: "work.handoff").order(:id).last
    assert_equal users(:david).id, row.actor_id
    assert_equal "ChannelThread", row.target_type
    assert_equal @thread.id, row.target_id
    assert_equal "Bender Bot", row.details["to_owner"]
  end

  test "hand_off! enqueues the receiver webhook after commit" do
    assert_enqueued_with(job: Agent::EventWebhookJob) do
      @thread.hand_off!(sender: users(:david), receiver_agent: @agent, summary: "Yours now")
    end

    event = @agent.agent_events.order(:id).last
    assert_equal "pending", event.reload.webhook_status
  end

  test "hand_off! rejects the current owner" do
    @thread.update_work!(actor: users(:david), work_owner_id: users(:bender).id)

    error = assert_raises(ActiveRecord::RecordInvalid) do
      @thread.hand_off!(sender: users(:david), receiver_agent: @agent, summary: "Again")
    end
    assert_includes error.record.errors[:work_owner], "is already the owner of this work"
  end

  test "hand_off! refuses an agent sender that no longer owns the thread" do
    @thread.update_work!(actor: users(:david), work_owner_id: users(:bender).id)
    @thread.update_work!(actor: users(:david), work_owner_id: users(:jz).id)

    assert_raises(ActiveRecord::RecordNotFound) do
      @thread.hand_off!(sender: users(:bender), receiver_agent: @agent, summary: "Yours")
    end

    assert_equal users(:jz).id, @thread.reload.work_owner_id
    assert_empty WorkHandoff.where(channel_thread_id: @thread.id)
    assert_empty AuditLog.where(action: "work.handoff", target_id: @thread.id)
  end

  test "hand_off! refuses a human sender who can no longer manage the thread" do
    @thread.update_work!(actor: users(:david), work_owner_id: users(:jz).id)
    @board.memberships.revoke_from(users(:jz))

    assert_raises(ChannelThread::WorkUpdateForbidden) do
      @thread.hand_off!(sender: users(:jz), receiver_agent: @agent, summary: "Yours")
    end

    assert_equal users(:jz).id, @thread.reload.work_owner_id
    assert_empty WorkHandoff.where(channel_thread_id: @thread.id)
  end

  test "hand_off! rejects untracked threads" do
    plain = ChannelThread.create!(room: rooms(:designers), creator: users(:david), name: "Chat")

    assert_raises(ActiveRecord::RecordNotFound) do
      plain.hand_off!(sender: users(:david), receiver_agent: @agent, summary: "Nope")
    end
  end

  test "hand_off! rejects an over-capped package" do
    assert_raises(ActiveRecord::RecordInvalid) do
      @thread.hand_off!(sender: users(:david), receiver_agent: @agent, summary: "x" * (WorkHandoff::SUMMARY_LIMIT + 1))
    end

    assert_equal users(:david).id, @thread.reload.work_owner_id
  end

  test "hand_off! validates receiver ownership eligibility on save" do
    AgentGrant.where(agent: @agent, capability: "post_messages").update_all(revoked_at: Time.current)

    assert_raises(ActiveRecord::RecordInvalid) do
      @thread.hand_off!(sender: users(:david), receiver_agent: @agent, summary: "Nope")
    end

    assert_equal users(:david).id, @thread.reload.work_owner_id
  end

  private
    def grant!(agent, capability)
      AgentGrant.create!(agent: agent, room: @board, granted_by: users(:david), capability: capability)
    end
end
