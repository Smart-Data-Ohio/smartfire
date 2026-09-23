require "test_helper"

class ChannelThreadAutoAssignTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz), users(:kevin), users(:bender) ])
    grant!("read_messages")
    grant!("post_messages")
  end

  test "a new tag auto-assigns an unowned post to a person" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned", tags: "bug")

    assert_equal users(:jz).id, post.reload.work_owner_id

    event = post.work_thread_events.ordered.first
    assert_equal "work_assignment", event.event_type
    assert_nil event.actor_id
    assert_equal "Auto-assigned by board tag rule", event.note
  end

  test "adding a tag later auto-assigns too" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned")
    assert_nil post.reload.work_owner_id

    post.tag_names = "bug"
    post.save!

    assert_equal users(:jz).id, post.reload.work_owner_id
  end

  test "auto-assign to an agent writes its work_assigned ledger row" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:bender), created_by: users(:david))

    ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned", tags: "bug")

    event = agents(:bender_agent).agent_events.deliverable.order(:id).last
    assert_equal "work_assigned", event.event_type
  end

  test "auto-assign never overrides a human assignment" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned", owner_id: users(:kevin).id)

    post.tag_names = "bug"
    post.save!

    assert_equal users(:kevin).id, post.reload.work_owner_id
  end

  test "auto-assign never overrides an agent assignment" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned", owner_id: users(:bender).id)

    post.tag_names = "bug"
    post.save!

    assert_equal users(:bender).id, post.reload.work_owner_id
  end

  test "auto-assign skips a rule whose assignee left the board" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    @board.memberships.revoke_from(users(:jz))

    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned", tags: "bug")

    assert_nil post.reload.work_owner_id
  end

  test "auto-assign skips an agent that lost read_messages" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:bender), created_by: users(:david))
    AgentGrant.where(agent: agents(:bender_agent), capability: "read_messages").update_all(revoked_at: Time.current)

    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned", tags: "bug")

    assert_nil post.reload.work_owner_id
  end

  test "auto-assign ignores channel threads" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    thread = ChannelThread.create!(room: rooms(:designers), creator: users(:david), name: "Chat", work_status: "planned")
    thread.tag_names = "bug"
    thread.save!

    assert_nil thread.reload.work_owner_id
  end

  test "work_status_changed_at stamps on every status change" do
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Broken", work_status: "planned")
    assert_in_delta Time.current, post.reload.work_status_changed_at, 5

    travel_to 2.hours.from_now do
      post.update_work!(actor: users(:david), work_status: "in_progress")
      assert_in_delta Time.current, post.reload.work_status_changed_at, 5
    end

    stamped = post.reload.work_status_changed_at

    travel_to 3.hours.from_now do
      post.tag_names = "bug"
      post.save!
      assert_equal stamped, post.reload.work_status_changed_at
    end
  end

  private
    def grant!(capability)
      AgentGrant.create!(agent: agents(:bender_agent), room: @board, granted_by: users(:david), capability: capability)
    end
end
