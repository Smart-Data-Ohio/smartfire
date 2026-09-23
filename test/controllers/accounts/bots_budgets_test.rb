require "test_helper"

class Accounts::BotsBudgetsTest < ActionDispatch::IntegrationTest
  setup do
    @bot = users(:bender)
    @agent = agents(:bender_agent)
  end

  test "edit page shows today's usage, budget fields, and the kill switch" do
    sign_in :david
    rooms(:watercooler).root_messages.create!(creator: @bot,
      markdown_source: "Posted", client_message_id: "bots-budget-usage")

    get edit_account_bot_url(@bot)

    assert_response :success
    assert_includes response.body, "Today's usage: 1 messages · 0 board posts · 0 external actions"
    assert_includes response.body, "Messages per day"
    assert_includes response.body, "Board posts per day"
    assert_includes response.body, "External actions per day"
    assert_includes response.body, "Kill switch: suspend agent"
  end

  test "administrator sets daily caps" do
    sign_in :david

    assert_difference "AuditLog.where(action: 'agent.update').count", 1 do
      patch account_bot_url(@bot), params: {
        user: { name: @bot.name },
        agent: { daily_message_cap: "50", daily_board_post_cap: "10", daily_external_action_cap: "5" }
      }
    end

    assert_redirected_to account_bots_url
    assert_equal 50, @agent.reload.daily_message_cap
    assert_equal 10, @agent.daily_board_post_cap
    assert_equal 5, @agent.daily_external_action_cap

    row = AuditLog.where(action: "agent.update").last
    assert_equal [ nil, 50 ], [ row.details["daily_message_cap"]["before"], row.details["daily_message_cap"]["after"] ]
  end

  test "owner without admin rights sets daily caps" do
    @agent.update!(owner: users(:kevin))
    sign_in users(:kevin)

    patch account_bot_url(@bot), params: {
      user: { name: @bot.name },
      agent: { daily_message_cap: "25" }
    }

    assert_redirected_to account_bots_url
    assert_equal 25, @agent.reload.daily_message_cap
  end

  test "invalid caps re-render the form" do
    sign_in :david

    patch account_bot_url(@bot), params: {
      user: { name: @bot.name },
      agent: { daily_message_cap: "0" }
    }

    assert_response :unprocessable_entity
    assert_nil @agent.reload.daily_message_cap
  end

  test "kill switch suspends, cancels approvals, and audits" do
    sign_in :david
    approval = AgentApproval.create!(agent: @agent, action: "deploy", summary: "Ship it")

    assert_difference "AuditLog.where(action: 'agent.kill_switch').count", 1 do
      post kill_switch_account_bot_url(@bot)
    end

    assert_redirected_to edit_account_bot_url(@bot)
    assert_predicate @agent.reload, :suspended?
    assert_equal "cancelled", approval.reload.status
  end

  test "owner without admin rights hits the kill switch" do
    @agent.update!(owner: users(:kevin))
    sign_in users(:kevin)

    post kill_switch_account_bot_url(@bot)

    assert_redirected_to edit_account_bot_url(@bot)
    assert_predicate @agent.reload, :suspended?
  end

  test "strangers cannot edit budgets or hit the kill switch" do
    sign_in users(:kevin)

    get edit_account_bot_url(@bot)
    assert_response :forbidden

    post kill_switch_account_bot_url(@bot)
    assert_response :forbidden
    assert_not @agent.reload.suspended?
  end

  test "kill switch is 404 for a legacy bot without an agent" do
    sign_in :david
    @agent.delete

    post kill_switch_account_bot_url(@bot)

    assert_response :not_found
  end
end
