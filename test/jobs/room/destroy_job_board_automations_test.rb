require "test_helper"

class Room::DestroyJobBoardAutomationsTest < ActiveJob::TestCase
  test "destroying a board with automations and a posted digest leaves no orphans" do
    board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz) ])
    BoardTagAssignment.create!(room: board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    BoardSlaRule.create!(room: board, work_status: "in_progress", nudge_after_minutes: 60, escalate_after_minutes: 240)
    post = ChannelThread.create_board_post!(room: board, creator: users(:david),
      name: "Stuck migration", work_status: "in_progress", owner_id: users(:jz).id)
    post.update_columns(work_status_changed_at: 3.hours.ago)
    BoardAutomations::SlaDispatcher.dispatch_due!
    BoardAutomations::DigestDispatcher.dispatch_due!
    assert_not_empty BoardSlaNudge.where(room_id: board.id)
    assert_not_nil BoardStaleDigest.order(:id).last.message_id

    board.begin_destroy!
    Room::DestroyJob.perform_now(board.id)

    assert_empty Room.where(id: board.id)
    assert_empty Message.where(room_id: board.id)
    assert_empty ChannelThread.where(room_id: board.id)
    assert_empty BoardTagAssignment.where(room_id: board.id)
    assert_empty BoardSlaRule.where(room_id: board.id)
    assert_empty BoardSlaNudge.where(room_id: board.id)
    assert_empty BoardStaleDigest.where(room_id: board.id)
    assert_empty ActivityItem.where(event_type: "work_sla")
  end
end
