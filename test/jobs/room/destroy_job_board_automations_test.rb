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

  test "destroying a board with digests, scheduled messages, and polls leaves no orphans" do
    board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz) ])
    BoardSlaRule.create!(room: board, work_status: "in_progress", nudge_after_minutes: 60, escalate_after_minutes: 240)
    post = ChannelThread.create_board_post!(room: board, creator: users(:david),
      name: "Stuck migration", work_status: "in_progress", owner_id: users(:jz).id)
    post.update_columns(work_status_changed_at: 3.hours.ago)
    BoardAutomations::SlaDispatcher.dispatch_due!
    BoardAutomations::DigestDispatcher.dispatch_due!
    assert_not_nil BoardStaleDigest.order(:id).last.message_id

    reply = post.post_message!(creator: users(:david), attributes: { markdown_source: "Which option?" })
    poll = Poll.create_for_message!(message: reply, labels: [ "A", "B" ])
    poll.cast_vote!(users(:jz), [ poll.poll_options.first.id ])
    pending = ScheduledMessage.create!(user: users(:david), room: board, thread: post,
      markdown_source: "Threaded nudge", send_at: 1.hour.from_now)
    dropped = ScheduledMessage.create!(user: users(:david), room: board,
      markdown_source: "Root post", send_at: 1.hour.from_now)
    dropped.drop!(reason: "test")
    scheduled_ids = [ pending.id, dropped.id ]
    item_ids = ActivityItem.where(source: [ pending, dropped ]).ids
    assert_not_empty item_ids

    board.begin_destroy!
    Room::DestroyJob.perform_now(board.id)

    assert_empty Room.where(id: board.id)
    assert_empty Message.where(room_id: board.id)
    assert_empty ChannelThread.where(room_id: board.id)
    assert_empty BoardStaleDigest.where(room_id: board.id)
    assert_empty ScheduledMessage.where(id: scheduled_ids)
    assert_empty Poll.where(id: poll.id)
    assert_empty PollOption.where(poll_id: poll.id)
    assert_empty PollVote.where(poll_id: poll.id)
    assert_empty ActivityItem.where(id: item_ids)
  end
end
