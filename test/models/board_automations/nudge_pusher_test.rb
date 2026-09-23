require "test_helper"

class BoardAutomations::NudgePusherTest < ActiveSupport::TestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz) ])
    @post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Stale work", work_status: "in_progress", owner_id: users(:jz).id)
    @nudge = BoardSlaNudge.create!(
      room: @board, channel_thread: @post, work_status: "in_progress",
      stage: "nudge", status_entered_at: 2.hours.ago, recipient: users(:jz)
    )
  end

  test "pushes the nudge payload to the recipient subscriptions" do
    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, subscriptions|
      payload.fetch(:title) == "Launch" &&
        payload.fetch(:body) == "SLA breach: Stale work sitting in In progress" &&
        payload.fetch(:path) == Rails.application.routes.url_helpers.room_path(@board, thread: @post.id) &&
        subscriptions.map(&:user_id) == [ users(:jz).id ]
    end

    BoardAutomations::NudgePusher.new(nudge: @nudge).push
  end

  test "escalations push with the escalated prefix" do
    @nudge.update!(stage: "escalation", recipient: users(:david))

    pool = Rails.configuration.x.web_push_pool
    pool.expects(:queue).with do |payload, _subscriptions|
      payload.fetch(:body) == "Escalated: Stale work sitting in In progress"
    end

    BoardAutomations::NudgePusher.new(nudge: @nudge).push
  end

  test "skips a recipient who left the board" do
    @board.memberships.revoke_from(users(:jz))

    Rails.configuration.x.web_push_pool.expects(:queue).never

    BoardAutomations::NudgePusher.new(nudge: @nudge).push
  end

  test "dnd silences the push like other reminders" do
    users(:jz).update!(dnd_enabled: true)

    Rails.configuration.x.web_push_pool.expects(:queue).never

    BoardAutomations::NudgePusher.new(nudge: @nudge).push
  end
end
