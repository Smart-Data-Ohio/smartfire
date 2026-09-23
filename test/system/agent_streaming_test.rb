require "application_system_test_case"

class AgentStreamingTest < ApplicationSystemTestCase
  setup do
    @forgery_protection = ActionController::Base.allow_forgery_protection
    ActionController::Base.allow_forgery_protection = true
    page.current_window.resize_to(1440, 1000)
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
    @bot = users(:bender)
  end

  teardown do
    ActionController::Base.allow_forgery_protection = @forgery_protection
    page.current_window.resize_to(1400, 1400)
  end

  test "streaming message renders, updates live, and finalizes" do
    sign_in "david@37signals.com"
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Drafting", client_message_id: "sys-stream-live")
    join_room @room

    within_message(message) do
      assert_selector ".message__streaming", text: "Working…"
      assert_text "Drafting"
    end

    message.update!(markdown_source: "Drafting more")
    message.broadcast_stream_update

    within_message(message) do
      assert_text "Drafting more"
      assert_selector ".message__streaming", text: "Working…"
    end

    message.finalize_stream!

    within_message(message) do
      assert_text "Drafting more"
      assert_no_selector ".message__streaming"
    end
  end

  test "agent steps render as a collapsible list" do
    sign_in "david@37signals.com"
    message = @room.root_messages.create!(creator: @bot,
      markdown_source: "Working on it", client_message_id: "sys-steps")
    AgentStep.create!(agent: @agent, message: message, name: "Run tests",
      status: "done", output_summary: "All green", duration_ms: 1500)
    AgentStep.create!(agent: @agent, message: message, name: "Deploy",
      status: "running", input_summary: "Ship it")
    join_room @room

    within_message(message) do
      assert_selector "details.agent-steps summary", text: "Steps (2)"
      find("details.agent-steps > summary").click
      assert_text "Run tests"
      assert_text "Deploy"
      assert_text "Done"
      assert_text "All green"
      assert_text "Ship it"
    end
  end

  test "working presence shows in the member panel" do
    sign_in "david@37signals.com"
    @agent.set_working_presence!("Running tests…")
    join_room @room

    # The panel opens itself on desktop widths; the presence text loads
    # with the member list.
    assert_selector "#channel-members [data-member-id='#{@bot.id}']", text: "Running tests…", wait: 20
  end

  test "owner sees budgets, one budget item, and hits the kill switch" do
    @agent.update!(owner: users(:kevin), daily_message_cap: 1, daily_board_post_cap: 10)
    sign_in "kevin@37signals.com"

    visit edit_account_bot_url(@bot)

    assert_text "Today's usage: 0/1 messages · 0/10 board posts"
    assert_field "Board posts per day", with: "10"

    @room.root_messages.create!(creator: @bot,
      markdown_source: "Only post", client_message_id: "sys-budget-post")
    2.times { Agents::Budgets.check(@agent.reload, :messages) }

    visit activity_items_url

    assert_selector ".activity-item", text: "Budget exceeded", count: 1
    assert_text "hit its daily messages budget"

    visit edit_account_bot_url(@bot)
    accept_confirm { click_button "Kill switch: suspend agent" }

    assert_text "Agent suspended"
    assert_no_button "Kill switch: suspend agent"
    assert @agent.reload.suspended?
  end
end
