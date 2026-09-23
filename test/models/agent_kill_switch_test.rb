require "test_helper"

class AgentKillSwitchTest < ActiveSupport::TestCase
  setup do
    @agent = agents(:bender_agent)
    @owner = @agent.owner
  end

  test "kill switch suspends, cancels pending approvals, and audits" do
    pending = AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")
    expired = AgentApproval.create!(agent: @agent, action: "old", summary: "Stale")
    expired.update_columns(expires_at: 1.hour.ago)
    decided = AgentApproval.create!(agent: @agent, action: "done", summary: "Over")
    decided.update_columns(status: "approved", decided_by_id: @owner.id, decided_at: Time.current)
    @agent.set_working_presence!("Thinking…")

    cancelled = nil
    assert_difference "AuditLog.where(action: %w[ agent.suspend agent.kill_switch ]).count", 2 do
      cancelled = @agent.kill_switch!
    end

    assert_equal 1, cancelled
    assert_predicate @agent.reload, :suspended?
    assert_equal "cancelled", pending.reload.status
    assert_equal "expired", expired.reload.status
    assert_equal "approved", decided.reload.status
    assert_nil @agent.working_presence
    assert_predicate @agent.agent_grants.active, :empty?

    row = AuditLog.where(action: "agent.kill_switch").sole
    assert_equal 1, row.details["pending_approvals_cancelled"]
  end

  test "kill switch marks cancelled approvals' inbox items handled" do
    approval = AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")
    item = ActivityItem.where(source: approval, user: @owner).sole

    @agent.kill_switch!

    assert_predicate item.reload, :handled?
  end

  test "kill switch blocks approved-but-unexecuted external actions" do
    approval = AgentApproval.create!(agent: @agent, action: "github.comment", summary: "Comment",
      room: rooms(:watercooler))
    approval.update_columns(status: "approved", decided_by_id: @owner.id, decided_at: Time.current)

    @agent.kill_switch!

    Github::PerformAgentActionJob.perform_now(approval.id)

    event = @agent.agent_events.where(event_type: "github_action_completed").sole
    assert_equal "failed", event.metadata["status"]
    assert_equal "Agent is suspended or deactivated", event.metadata["message"]
  end

  test "kill switch is safe to repeat" do
    @agent.kill_switch!

    assert_nothing_raised do
      @agent.kill_switch!
    end
  end

  test "kill switch quietly finalizes the agent's open streams" do
    room = rooms(:watercooler)
    message = room.root_messages.create!(creator: @agent.user, streaming: true,
      markdown_source: "Hey @[David] hovercraft", client_message_id: "kill-stream")
    membership = Membership.find_by!(room: room, user: users(:david))

    assert_no_enqueued_jobs do
      @agent.kill_switch!
    end

    assert_not message.reload.streaming?
    assert_empty ActivityItem.where(source: message)
    assert_empty @agent.agent_events.where(message_id: message.id)
    assert_equal [], room.messages.search("hovercraft")
    assert_nil membership.reload.unread_at
    assert_empty ActionCable.server.pubsub.broadcasts(UnreadRoomsChannel.stream_name_for(users(:david).id))

    stream = [ room.to_gid_param, :messages ].join(":")
    assert_equal 1, ActionCable.server.pubsub.broadcasts(stream).size
  end

  test "suspend quietly finalizes the agent's open streams" do
    room = rooms(:watercooler)
    message = room.root_messages.create!(creator: @agent.user, streaming: true,
      markdown_source: "Hey @[David] hovercraft", client_message_id: "suspend-stream")

    assert_no_enqueued_jobs do
      @agent.suspend!
    end

    assert_not message.reload.streaming?
    assert_empty ActivityItem.where(source: message)
    assert_empty @agent.agent_events.where(message_id: message.id)
    assert_equal [], room.messages.search("hovercraft")
  end

  test "a suspension that rolls back leaves open streams untouched" do
    room = rooms(:watercooler)
    message = room.root_messages.create!(creator: @agent.user, streaming: true,
      markdown_source: "Still thinking", client_message_id: "rollback-stream")

    assert_no_turbo_stream_broadcasts [ room, :messages ] do
      Agent.transaction do
        @agent.suspend!
        raise ActiveRecord::Rollback
      end
    end

    assert message.reload.streaming?
    assert_not @agent.reload.suspended?
  end

  test "kill switch leaves owned work assigned without reassigning it" do
    board = Rooms::Board.create_for({ name: "Work Board", creator: users(:david) },
      users: [ users(:david), @agent.user ])
    AgentGrant.create!(agent: @agent, room: board, granted_by: users(:david), capability: "post_messages")
    thread = ChannelThread.create_board_post!(room: board, creator: users(:david),
      name: "Ship it", work_status: "in_progress", owner_id: @agent.user.id)

    assert_no_difference -> { thread.work_thread_events.count } do
      @agent.kill_switch!
    end

    assert_equal @agent.user.id, thread.reload.work_owner_id
    assert_equal "in_progress", thread.work_status
    assert_not thread.work_owner_active?
  end
end
