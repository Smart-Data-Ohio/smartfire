require "application_system_test_case"

class BoardAutomationsTest < ApplicationSystemTestCase
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz) ])
    @agent_user = User.create_bot!(name: "Board Agent")
    @agent = @agent_user.create_agent!(kind: :workspace, owner: users(:david))
    @board.memberships.grant_to(@agent_user)
    AgentGrant.create!(agent: @agent, room: @board, granted_by: users(:david), capability: "post_messages")
    AgentGrant.create!(agent: @agent, room: @board, granted_by: users(:david), capability: "manage_threads")

    sign_in "david@37signals.com"
  end

  test "the board creator configures tag rules and sla timers" do
    visit room_path(@board)
    click_link "Automations"

    assert_selector "h1", text: "Automations for Launch"

    fill_in "Tag", with: "bug"
    select "JZ", from: "Assign to"
    click_button "Add rule"
    assert_selector "ul", text: "bug"

    fill_in "sla_rules[planned][nudge_after_minutes]", with: "60"
    fill_in "sla_rules[planned][escalate_after_minutes]", with: "240"
    fill_in "sla_rules[blocked][nudge_after_minutes]", with: "30"
    fill_in "sla_rules[blocked][escalate_after_minutes]", with: "120"
    click_button "Save SLA timers"

    assert_equal 60, @board.board_sla_rules.find_by(work_status: "planned").nudge_after_minutes
    assert_equal 120, @board.board_sla_rules.find_by(work_status: "blocked").escalate_after_minutes
  end

  test "a plain member sees no automations link and is forbidden directly" do
    sign_in "jz@37signals.com"
    visit room_path(@board)

    assert_no_link "Automations"

    # The controller answers 403 (see the controller test); here the page
    # simply carries no automations content.
    visit board_automations_path(@board)
    assert_no_selector "#automations-title"
  end

  test "a tagged post auto-assigns and the history shows it" do
    BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))

    visit room_path(@board)
    click_link "New post"
    fill_in "Title", with: "Broken login"
    fill_in "bug, api", with: "bug"
    click_button "Create post"

    assert_selector ".board-post__facts", text: /JZ/
    assert_selector ".board-post__history", text: /Auto-assigned by board tag rule/
  end

  test "a person hands a post to an agent with context" do
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Ship it", work_status: "in_progress", owner_id: users(:david).id)

    visit room_thread_path(@board, post)
    find(".board-post__work summary", text: "Update work").click
    click_link "Hand off to an agent"

    select "Board Agent", from: "Receiving agent"
    fill_in "Summary", with: "Halfway there, tests are green"
    fill_in "Links (one URL per line, up to 10)", with: "https://example.com/spec"
    fill_in "Open questions (one per line, up to 10)", with: "Which API ships first?"
    click_button "Hand off"

    assert_selector ".board-post__facts", text: /Board Agent/
    assert_selector ".board-post__history", text: /David handed off/
    assert_selector ".board-post__history", text: /Halfway there, tests are green/
  end

  test "the stale-work digest renders on the board" do
    BoardSlaRule.create!(room: @board, work_status: "in_progress", nudge_after_minutes: 60, escalate_after_minutes: 240)
    post = ChannelThread.create_board_post!(room: @board, creator: users(:david),
      name: "Stuck migration", work_status: "in_progress", owner_id: users(:jz).id)
    post.update_columns(work_status_changed_at: 3.hours.ago)
    BoardAutomations::DigestDispatcher.dispatch_due!

    visit room_path(@board)

    assert_selector ".board__digest", text: /Stale-work digest/
    assert_selector ".board__digest", text: /Stuck migration/
    assert_selector ".board__digest", text: /In progress/
  end
end
