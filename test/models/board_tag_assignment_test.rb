require "test_helper"

class BoardTagAssignmentTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz), users(:kevin) ])
  end

  test "assigns to a human member" do
    assignment = BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))

    assert_equal "bug", assignment.tag
  end

  test "normalises the tag" do
    assignment = BoardTagAssignment.create!(room: @board, tag: "  Bug ", assignee: users(:jz), created_by: users(:david))

    assert_equal "bug", assignment.reload.tag
  end

  test "assigns to an eligible agent member" do
    agent = make_board_agent!(users(:bender))

    assignment = BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:bender), created_by: users(:david))

    assert_equal agent.user_id, assignment.assignee_id
  end

  test "rejects a tag outside the thread tag format" do
    assignment = BoardTagAssignment.new(room: @board, tag: "Not A Tag!", assignee: users(:jz), created_by: users(:david))

    assert_not assignment.valid?
    assert assignment.errors[:tag].any?
  end

  test "rejects a duplicate tag on the same board" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    duplicate = BoardTagAssignment.new(room: @board, tag: "BUG", assignee: users(:kevin), created_by: users(:david))

    assert_not duplicate.valid?
    assert duplicate.errors[:tag].any?
  end

  test "allows the same tag on another board" do
    other = Rooms::Board.create_for({ name: "Other", creator: users(:david) }, users: [ users(:david), users(:jz) ])
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))

    assert BoardTagAssignment.new(room: other, tag: "bug", assignee: users(:jz), created_by: users(:david)).valid?
  end

  test "rejects a non-board room" do
    assignment = BoardTagAssignment.new(room: rooms(:watercooler), tag: "bug", assignee: users(:david), created_by: users(:david))

    assert_not assignment.valid?
    assert_includes assignment.errors[:room], "must be a board"
  end

  test "rejects an assignee outside the board" do
    assignment = BoardTagAssignment.new(room: @board, tag: "bug", assignee: users(:kevin), created_by: users(:david))
    @board.memberships.revoke_from(users(:kevin))

    assert_not assignment.valid?
    assert_includes assignment.errors[:assignee], "must be an active board member able to own posts"
  end

  test "rejects a suspended agent assignee" do
    make_board_agent!(users(:bender))
    agents(:bender_agent).update!(suspended_at: Time.current)

    assignment = BoardTagAssignment.new(room: @board, tag: "bug", assignee: users(:bender), created_by: users(:david))

    assert_not assignment.valid?
    assert_includes assignment.errors[:assignee], "must be an active board member able to own posts"
  end

  test "rejects an agent assignee without post_messages" do
    @board.memberships.grant_to(users(:bender))
    AgentGrant.create!(agent: agents(:bender_agent), room: @board, granted_by: users(:david), capability: "read_messages")

    assignment = BoardTagAssignment.new(room: @board, tag: "bug", assignee: users(:bender), created_by: users(:david))

    assert_not assignment.valid?
    assert_includes assignment.errors[:assignee], "must be an active board member able to own posts"
  end

  private
    def make_board_agent!(user)
      @board.memberships.grant_to(user)
      AgentGrant.create!(agent: user.agent, room: @board, granted_by: users(:david), capability: "post_messages")
      user.agent
    end
end
