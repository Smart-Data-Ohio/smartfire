require "application_system_test_case"

class SlashCommandsTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "typing slash opens the command picker with combobox semantics" do
    editor = find_field("Write a message")
    editor.set("/")

    assert_selector "suggestion-option", text: "/poll"
    assert_selector "suggestion-option", text: "/huddle"
    assert_equal "list", editor["aria-autocomplete"]

    editor.set("/po")
    assert_selector "suggestion-option", text: "/poll"
    assert_no_selector "suggestion-option", text: "/huddle"

    editor.send_keys(:enter)
    assert_field "Write a message", with: "/poll "
  end

  test "the picker lists registered agent commands" do
    @room.memberships.grant_to users(:bender)
    AgentSlashCommand.create!(agent: agents(:bender_agent), room: @room, name: "deploy", description: "Ship it")

    find_field("Write a message").set("/dep")

    assert_selector "suggestion-option", text: "/deploy"
    assert_selector "suggestion-option", text: "Bender Bot"
  end

  test "shrug posts through the picker" do
    editor = find_field("Write a message")
    editor.set("/shrug")
    assert_selector "suggestion-option", text: "/shrug"
    editor.send_keys(:enter)
    assert_field "Write a message", with: "/shrug "
    editor.send_keys("ship it")
    editor.send_keys(:enter)

    assert_message_text "ship it", wait: 10
    assert_field "Write a message", with: ""
  end

  test "unknown commands show an error and keep the draft" do
    fill_in_markdown "message_markdown_source", with: "/frobnicate now"
    click_on "Send Message"

    assert_selector "#composer .composer__feedback", text: "Unknown command", visible: true
    assert_field "Write a message", with: "/frobnicate now"
    assert_no_selector ".message__body", text: "frobnicate"
  end

  test "me renders as an action line" do
    fill_in_markdown "message_markdown_source", with: "/me is reviewing the deploy"
    click_on "Send Message"

    assert_selector ".message--action .message__body", text: "is reviewing the deploy", wait: 10
    assert_field "Write a message", with: ""
  end

  test "poll opens the poll builder" do
    fill_in_markdown "message_markdown_source", with: "/poll"
    click_on "Send Message"

    assert_selector "#poll-builder[open]", visible: true
    assert_field "Write a message", with: ""
  end

  test "event navigates to the prefilled form" do
    fill_in_markdown "message_markdown_source", with: "/event Launch party"
    click_on "Send Message"

    assert_selector "h1", text: "Schedule an event"
    assert_field "Title", with: "Launch party"
  end

  test "huddle reports when unconfigured" do
    fill_in_markdown "message_markdown_source", with: "/huddle"
    click_on "Send Message"

    # No LiveKit environment in this suite, so the command errors ephemerally.
    assert_selector "#composer .composer__feedback", text: "not configured", visible: true
  end

  test "agent commands respond ephemerally until the agent replies" do
    @room.memberships.grant_to users(:bender)
    AgentSlashCommand.create!(agent: agents(:bender_agent), room: @room, name: "deploy")

    fill_in_markdown "message_markdown_source", with: "/deploy staging"
    click_on "Send Message"

    assert_selector "#composer .composer__feedback", text: "Sent to Bender Bot", visible: true
    assert_field "Write a message", with: ""
    assert_no_selector ".message__body", text: "deploy staging"
    assert_equal "staging", agents(:bender_agent).agent_events.ordered.last.metadata["arguments"]
  end

  test "status sets the custom status" do
    fill_in_markdown "message_markdown_source", with: "/status 🚂 On a train"
    click_on "Send Message"

    assert_selector "#composer .composer__feedback", text: "Status set", visible: true
    assert_equal "🚂", users(:jz).reload.custom_status_emoji
  end
end
