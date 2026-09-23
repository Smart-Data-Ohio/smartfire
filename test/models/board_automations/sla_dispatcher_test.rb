require "test_helper"

class BoardAutomations::SlaDispatcherTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz), users(:kevin) ])
    BoardSlaRule.create!(room: @board, work_status: "in_progress", nudge_after_minutes: 60, escalate_after_minutes: 240)
  end

  test "a post past the nudge threshold notifies its owner once" do
    post = stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 2.hours)

    assert_enqueued_with(job: BoardAutomations::NudgePushJob) do
      assert_difference -> { BoardSlaNudge.count }, 1 do
        BoardAutomations::SlaDispatcher.dispatch_due!
      end
    end

    nudge = BoardSlaNudge.order(:id).last
    assert_equal "nudge", nudge.stage
    assert_equal users(:jz).id, nudge.recipient_id
    assert_equal "in_progress", nudge.work_status

    item = ActivityItem.where(user: users(:jz), source: nudge).sole
    assert_equal "work_sla", item.event_type

    assert_no_enqueued_jobs(only: BoardAutomations::NudgePushJob) do
      assert_no_difference -> { BoardSlaNudge.count } do
        BoardAutomations::SlaDispatcher.dispatch_due!
      end
    end
    assert_equal post.id, nudge.channel_thread_id
  end

  test "a repeat sweep opens no write transaction for claimed crossings" do
    stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 5.hours)
    BoardAutomations::SlaDispatcher.dispatch_due!
    assert_equal 2, BoardSlaNudge.count

    statements = []
    callback = ->(_name, _start, _finish, _id, payload) { statements << payload[:sql] }
    ActiveSupport::Notifications.subscribed(callback, "sql.active_record") do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end

    writes = statements.grep(/\A\s*(begin|commit|rollback|insert|update|delete)/i)
    assert_empty writes
  end

  test "a post past the escalation threshold escalates to the board creator" do
    stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 5.hours)

    BoardAutomations::SlaDispatcher.dispatch_due!

    stages = BoardSlaNudge.order(:id).pluck(:stage, :recipient_id)
    assert_equal [ [ "nudge", users(:jz).id ], [ "escalation", users(:david).id ] ], stages
    assert_equal 1, ActivityItem.where(user: users(:david), event_type: "work_sla").count
  end

  test "the escalation fires on a later sweep when only the nudge was due" do
    post = stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 2.hours)

    travel_to 30.minutes.from_now do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end
    assert_equal [ "nudge" ], BoardSlaNudge.pluck(:stage)

    travel_to 6.hours.from_now do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end
    assert_equal %w[ escalation nudge ], BoardSlaNudge.pluck(:stage).sort
    assert_equal post.id, BoardSlaNudge.order(:id).last.channel_thread_id
  end

  test "a status change resets the timers for the new crossing" do
    post = stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 2.hours)
    BoardAutomations::SlaDispatcher.dispatch_due!
    assert_equal 1, BoardSlaNudge.count

    travel_to 1.hour.from_now do
      post.update_work!(actor: users(:david), work_status: "blocked")
      BoardSlaRule.create!(room: @board, work_status: "blocked", nudge_after_minutes: 30, escalate_after_minutes: 120)
    end

    travel_to 2.hours.from_now do
      assert_difference -> { BoardSlaNudge.count }, 1 do
        BoardAutomations::SlaDispatcher.dispatch_due!
      end
    end

    assert_equal "blocked", BoardSlaNudge.order(:id).last.work_status
  end

  test "a post that left the status before the sweep fires nothing" do
    post = stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 2.hours)
    post.update_work!(actor: users(:david), work_status: "done")

    assert_no_difference -> { BoardSlaNudge.count } do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end
  end

  test "an agent-owned post nudges the agent's human owner" do
    @board.memberships.grant_to(users(:bender))
    grant!("post_messages")
    post = stale_post!(owner: users(:bender), status: "in_progress", entered_ago: 2.hours)

    BoardAutomations::SlaDispatcher.dispatch_due!

    nudge = BoardSlaNudge.order(:id).last
    assert_equal users(:david).id, nudge.recipient_id
    assert_equal post.id, nudge.channel_thread_id
  end

  test "an unassigned post nudges the board creator" do
    stale_post!(owner: nil, status: "in_progress", entered_ago: 2.hours)

    BoardAutomations::SlaDispatcher.dispatch_due!

    assert_equal users(:david).id, BoardSlaNudge.order(:id).last.recipient_id
  end

  test "a post under the threshold fires nothing" do
    stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 30.minutes)

    assert_no_difference -> { BoardSlaNudge.count } do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end
  end

  test "a post in an unruled status fires nothing" do
    stale_post!(owner: users(:jz), status: "planned", entered_ago: 3.days)

    assert_no_difference -> { BoardSlaNudge.count } do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end
  end

  test "a deleted board fires nothing" do
    post = stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 2.hours)
    @board.update_columns(deleted_at: Time.current)

    assert_no_difference -> { BoardSlaNudge.count } do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end
    assert post.reload
  end

  test "a revoked recipient loses the inbox item but the claim holds" do
    post = stale_post!(owner: users(:jz), status: "in_progress", entered_ago: 2.hours)
    BoardAutomations::SlaDispatcher.dispatch_due!
    assert_equal 1, ActivityItem.accessible_to(users(:jz)).where(event_type: "work_sla").count

    @board.memberships.revoke_from(users(:jz))

    assert_equal 0, ActivityItem.accessible_to(users(:jz)).where(event_type: "work_sla").count
    assert_no_difference -> { BoardSlaNudge.count } do
      BoardAutomations::SlaDispatcher.dispatch_due!
    end
    assert post.reload
  end

  private
    def stale_post!(owner:, status:, entered_ago:)
      post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
        name: "Stale #{status}", work_status: status, owner_id: owner&.id)
      post.update_columns(work_status_changed_at: entered_ago.ago)
      post
    end

    def grant!(capability)
      AgentGrant.create!(agent: agents(:bender_agent), room: @board, granted_by: users(:david), capability: capability)
    end
end
