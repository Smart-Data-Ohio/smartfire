require "test_helper"

class BoardSlaRuleTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz) ])
  end

  test "creates a rule per status" do
    rule = BoardSlaRule.create!(room: @board, work_status: "in_progress", nudge_after_minutes: 60, escalate_after_minutes: 240)

    assert_equal "in_progress", rule.work_status
  end

  test "covers the blocked status like any other" do
    rule = BoardSlaRule.create!(room: @board, work_status: "blocked", nudge_after_minutes: 30, escalate_after_minutes: 120)

    assert rule.valid?
  end

  test "rejects an unknown status" do
    rule = BoardSlaRule.new(room: @board, work_status: "shipped", nudge_after_minutes: 60, escalate_after_minutes: 240)

    assert_not rule.valid?
    assert rule.errors[:work_status].any?
  end

  test "rejects a duplicate status on the same board" do
    BoardSlaRule.create!(room: @board, work_status: "planned", nudge_after_minutes: 60, escalate_after_minutes: 240)
    duplicate = BoardSlaRule.new(room: @board, work_status: "planned", nudge_after_minutes: 30, escalate_after_minutes: 120)

    assert_not duplicate.valid?
    assert duplicate.errors[:work_status].any?
  end

  test "rejects non-positive and over-long thresholds" do
    [ 0, -5, BoardSlaRule::MAX_MINUTES + 1 ].each do |minutes|
      rule = BoardSlaRule.new(room: @board, work_status: "planned", nudge_after_minutes: minutes, escalate_after_minutes: 240)

      assert_not rule.valid?, "nudge #{minutes} should be invalid"
      assert rule.errors[:nudge_after_minutes].any?
    end
  end

  test "rejects an escalation at or before the nudge" do
    [ 60, 30 ].each do |escalate|
      rule = BoardSlaRule.new(room: @board, work_status: "planned", nudge_after_minutes: 60, escalate_after_minutes: escalate)

      assert_not rule.valid?, "escalate #{escalate} should be invalid"
      assert_includes rule.errors[:escalate_after_minutes], "must be after the nudge threshold"
    end
  end

  test "rejects a non-board room" do
    rule = BoardSlaRule.new(room: rooms(:watercooler), work_status: "planned", nudge_after_minutes: 60, escalate_after_minutes: 240)

    assert_not rule.valid?
    assert_includes rule.errors[:room], "must be a board"
  end
end
