require "test_helper"

class BoardAutomations::DigestDispatcherTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz) ])
    BoardSlaRule.create!(room: @board, work_status: "in_progress", nudge_after_minutes: 60, escalate_after_minutes: 240)
  end

  test "a stale post produces one quiet digest note" do
    stale_post!(name: "Old work", owner: users(:jz), entered_ago: 2.hours)
    fresh_post!

    assert_difference -> { BoardStaleDigest.count }, 1 do
      assert_difference -> { @board.messages.count }, 1 do
        BoardAutomations::DigestDispatcher.dispatch_due!
      end
    end

    digest = BoardStaleDigest.order(:id).last
    assert_equal Date.current, digest.digest_on

    note = digest.message
    assert_predicate note, :system_note?
    assert_nil note.thread_id
    assert_includes note.plain_text_body, "Stale work digest: 1 post past its SLA"
    assert_includes note.plain_text_body, "Old work"
    assert_includes note.plain_text_body, "In progress"
    assert_not_includes note.plain_text_body, "Fresh work"
  end

  test "the digest escapes titles and names instead of rendering markdown" do
    @board.memberships.grant_to(users(:kevin))
    users(:kevin).update!(name: "*Kevin*")
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "**Bold** title", work_status: "in_progress", owner_id: users(:kevin).id)
    post.update_columns(work_status_changed_at: 2.hours.ago)

    BoardAutomations::DigestDispatcher.dispatch_due!

    body = BoardStaleDigest.order(:id).last.message.plain_text_body
    assert_includes body, "**Bold** title"
    assert_includes body, "*Kevin*"
  end

  test "the digest fires once per day and again the next day" do
    stale_post!(name: "Old work", owner: users(:jz), entered_ago: 2.hours)

    BoardAutomations::DigestDispatcher.dispatch_due!
    assert_no_difference -> { BoardStaleDigest.count } do
      BoardAutomations::DigestDispatcher.dispatch_due!
    end

    travel_to 1.day.from_now do
      assert_difference -> { BoardStaleDigest.count }, 1 do
        BoardAutomations::DigestDispatcher.dispatch_due!
      end
    end
  end

  test "a repeat sweep is a silent no-op that logs no error" do
    stale_post!(name: "Old work", owner: users(:jz), entered_ago: 2.hours)
    BoardAutomations::DigestDispatcher.dispatch_due!
    assert_equal 1, BoardStaleDigest.count

    log = StringIO.new
    original_logger = Rails.logger
    Rails.logger = ActiveSupport::Logger.new(log)
    begin
      assert_no_difference -> { BoardStaleDigest.count } do
        assert_no_difference -> { @board.messages.count } do
          BoardAutomations::DigestDispatcher.dispatch_due!
        end
      end
    ensure
      Rails.logger = original_logger
    end

    assert_no_match "Board stale digest failed", log.string
  end

  test "no stale posts posts nothing and claims nothing" do
    fresh_post!

    assert_no_difference -> { BoardStaleDigest.count } do
      assert_no_difference -> { @board.messages.count } do
        BoardAutomations::DigestDispatcher.dispatch_due!
      end
    end
  end

  test "done posts never appear even with a done rule" do
    BoardSlaRule.create!(room: @board, work_status: "done", nudge_after_minutes: 60, escalate_after_minutes: 240)
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Finished", work_status: "done", owner_id: users(:jz).id)
    post.update_columns(work_status_changed_at: 3.days.ago)

    assert_no_difference -> { BoardStaleDigest.count } do
      BoardAutomations::DigestDispatcher.dispatch_due!
    end
  end

  test "boards without rules are skipped" do
    other = Rooms::Board.create_for({ name: "Other", creator: users(:david) }, users: [ users(:david) ])
    post = ChannelThread.create_board_post!(room: other, creator: users(:david),
      name: "Old work", work_status: "in_progress")
    post.update_columns(work_status_changed_at: 3.days.ago)

    assert_no_difference -> { BoardStaleDigest.count } do
      BoardAutomations::DigestDispatcher.dispatch_due!
    end
  end

  test "the digest note stays quiet: no inbox, unread, or agent delivery" do
    stale_post!(name: "Old work", owner: users(:jz), entered_ago: 2.hours)
    membership = @board.memberships.find_by(user: users(:jz))
    membership.update_columns(unread_at: nil)

    assert_no_difference -> { ActivityItem.count } do
      BoardAutomations::DigestDispatcher.dispatch_due!
    end

    assert_nil membership.reload.unread_at
  end

  private
    def stale_post!(name:, owner:, entered_ago:)
      post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
        name: name, work_status: "in_progress", owner_id: owner&.id)
      post.update_columns(work_status_changed_at: entered_ago.ago)
      post
    end

    def fresh_post!
      ChannelThread.create_board_post!(room: @board, creator: users(:david),
        name: "Fresh work", work_status: "in_progress", owner_id: users(:jz).id)
    end
end
