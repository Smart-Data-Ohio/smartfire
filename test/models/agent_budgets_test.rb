require "test_helper"

class AgentBudgetsTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
  end

  test "usage counts today's messages, board posts, and approvals" do
    @room.root_messages.create!(creator: @agent.user,
      markdown_source: "One", client_message_id: "budget-message")
    board = Rooms::Board.create_for({ name: "Budget Board", creator: users(:david) },
      users: [ users(:david), @agent.user ])
    ChannelThread.create_board_post!(room: board, creator: @agent.user,
      name: "Post", work_status: "planned")
    AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")

    assert_equal({ messages: 1, board_posts: 1, external_actions: 1 }, Agents::Budgets.usage(@agent))
  end

  test "a board post's first message counts only toward the board-post cap" do
    board = Rooms::Board.create_for({ name: "Opener Board", creator: users(:david) },
      users: [ users(:david), @agent.user ])
    ChannelThread.create_board_post!(room: board, creator: @agent.user,
      name: "Post", work_status: "planned", first_message: "Opening words")

    assert_equal({ messages: 0, board_posts: 1, external_actions: 0 }, Agents::Budgets.usage(@agent))
  end

  test "an opener does not burn the message budget but replies do" do
    @agent.update!(daily_message_cap: 1)
    board = Rooms::Board.create_for({ name: "Cap Board", creator: users(:david) },
      users: [ users(:david), @agent.user ])
    thread = ChannelThread.create_board_post!(room: board, creator: @agent.user,
      name: "Post", work_status: "planned", first_message: "Opening words")

    assert_nil Agents::Budgets.check(@agent, :messages)

    thread.post_message!(creator: @agent.user, attributes: {
      markdown_source: "A reply", client_message_id: "budget-board-reply" })

    denial = Agents::Budgets.check(@agent, :messages)

    assert_not_nil denial
    assert_equal :too_many_requests, denial.status
  end

  test "usage ignores yesterday's rows" do
    message = @room.root_messages.create!(creator: @agent.user,
      markdown_source: "Old", client_message_id: "budget-old")
    message.update_columns(created_at: 1.day.ago)

    assert_equal 0, Agents::Budgets.usage(@agent)[:messages]
  end

  test "unlimited caps always pass" do
    assert_nil Agents::Budgets.check(@agent, :messages)
    assert_nil Agents::Budgets.check(@agent, :board_posts)
    assert_nil Agents::Budgets.check(@agent, :external_actions)
  end

  test "a hit cap answers 429 and notifies the owner once per day" do
    @agent.update!(daily_message_cap: 1)
    @room.root_messages.create!(creator: @agent.user,
      markdown_source: "One", client_message_id: "budget-cap")

    denial = nil
    assert_difference -> { ActivityItem.where(event_type: "agent_budget_exceeded").count }, 1 do
      denial = Agents::Budgets.check(@agent, :messages)
    end

    assert_not_nil denial
    assert_not denial.ok?
    assert_equal :too_many_requests, denial.status
    assert_equal "Daily message budget exceeded (1/day)", denial.error
    assert_equal "messages", denial.payload[:cap]
    assert_equal 1, denial.payload[:limit]
    assert denial.payload[:retry_after].positive?

    item = ActivityItem.where(event_type: "agent_budget_exceeded").sole
    assert_equal @agent.owner_id, item.user_id

    assert_no_difference -> { ActivityItem.where(event_type: "agent_budget_exceeded").count } do
      repeat = Agents::Budgets.check(@agent, :messages)
      assert_equal :too_many_requests, repeat.status
    end
  end

  test "each cap notifies separately" do
    @agent.update!(daily_message_cap: 1, daily_board_post_cap: 1)
    @room.root_messages.create!(creator: @agent.user,
      markdown_source: "One", client_message_id: "budget-each")
    board = Rooms::Board.create_for({ name: "Each Board", creator: users(:david) },
      users: [ users(:david), @agent.user ])
    ChannelThread.create_board_post!(room: board, creator: @agent.user,
      name: "Post", work_status: "planned")

    Agents::Budgets.check(@agent, :messages)
    Agents::Budgets.check(@agent, :board_posts)

    assert_equal 2, ActivityItem.where(event_type: "agent_budget_exceeded").count
  end

  test "an ownerless agent notifies administrators" do
    @agent.update_columns(owner_id: nil)
    @agent.update!(daily_message_cap: 1)
    @room.root_messages.create!(creator: @agent.user,
      markdown_source: "One", client_message_id: "budget-ownerless")

    Agents::Budgets.check(@agent, :messages)

    recipients = ActivityItem.where(event_type: "agent_budget_exceeded").map(&:user_id)
    assert_equal User.active.without_bots.where(role: :administrator).ids.sort, recipients.sort
    assert_not_empty recipients
  end

  test "a stranger cannot read another agent's budget item" do
    @agent.update!(daily_message_cap: 1)
    @room.root_messages.create!(creator: @agent.user,
      markdown_source: "One", client_message_id: "budget-stranger")
    Agents::Budgets.check(@agent, :messages)

    assert_empty ActivityItem.accessible_to(users(:jason)).where(event_type: "agent_budget_exceeded")
    assert_equal 1, ActivityItem.accessible_to(@agent.owner).where(event_type: "agent_budget_exceeded").count
  end

  test "caps must be positive integers" do
    @agent.daily_message_cap = 0
    assert_not @agent.valid?

    @agent.daily_message_cap = -3
    assert_not @agent.valid?

    @agent.daily_message_cap = nil
    assert @agent.valid?
  end
end
