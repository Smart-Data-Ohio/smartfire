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

  test "arguments close the slash picker and Enter posts" do
    record_slash_command_fetches

    editor = find_field("Write a message")
    editor.set("/shrug")
    assert_selector "suggestion-option", text: "/shrug"

    # Replace the query with a command plus arguments without committing.
    # Either the debounced update runs and closes the picker (fixed), or
    # it re-queries for the whole line and stays active but hidden
    # (buggy) — wait for whichever proves the update ran.
    editor.set("/shrug ship it")
    page.document.synchronize(Capybara.default_max_wait_time) do
      fetched = page.evaluate_script("window.__slashCommandFetchUrls.some(url => url.includes('ship'))")
      closed = page.evaluate_script("document.querySelectorAll('suggestion-select').length === 0")
      raise Capybara::ElementNotFound unless fetched || closed
    end

    assert_no_selector "suggestion-option"
    editor.send_keys(:enter)

    assert_message_text "ship it", wait: 10
    assert_field "Write a message", with: ""
  end

  test "Enter submits while the picker's deactivating update is still pending" do
    editor = find_field("Write a message")
    editor.set("/shrug")
    assert_selector "suggestion-option", text: "/shrug"

    # Enter lands before the debounced update can run, while the stale
    # "shrug" suggestion is still showing: it must submit "/shrug ship
    # it", not commit the stale suggestion again. On the fixed code the
    # submit wins however the timing falls, so this stays green.
    editor.set("/shrug ship it")
    editor.send_keys(:enter)

    assert_message_text "ship it", wait: 10
    assert_field "Write a message", with: ""
  end

  test "a submit queued during the live command check still runs the command" do
    # Hold the live-list check open so the switch below lands while it is
    # pending; the completion must route the current draft, not the stale
    # word and not a literal post.
    page.execute_script <<~JS
      window.__slashCommandFetchUrls = [];
      const originalFetch = window.fetch;
      window.fetch = (input, init) => {
        const url = typeof input === "string" ? input : input.url;
        if (url.includes("/autocompletable/slash_commands")) {
          window.__slashCommandFetchUrls.push(url);
          return new Promise(resolve => setTimeout(() => resolve(originalFetch(input, init)), 2000));
        }
        return originalFetch(input, init);
      };
    JS

    fill_in_markdown "message_markdown_source", with: "/etc/hosts is not a command"
    click_on "Send Message"

    # The live check fetches the list URL with no query; wait for it so
    # the switch provably lands while the check is pending.
    page.document.synchronize(Capybara.default_max_wait_time) do
      urls = page.evaluate_script("window.__slashCommandFetchUrls")
      raise Capybara::ElementNotFound unless urls.any? { |url| !url.include?("query=") }
    end

    editor = find_field("Write a message")
    fill_in_markdown "message_markdown_source", with: "/shrug late switch"
    editor.send_keys(:enter)

    assert_message_text "(ツ)", wait: 10
    assert_no_selector ".message__body", text: "/shrug late switch"
    assert_field "Write a message", with: ""
  end

  test "unknown slash words post as normal messages" do
    fill_in_markdown "message_markdown_source", with: "/etc/hosts is not a command"
    click_on "Send Message"

    assert_message_text "/etc/hosts is not a command", wait: 10
    assert_field "Write a message", with: ""
  end

  test "double slash escapes a known command" do
    fill_in_markdown "message_markdown_source", with: "//poll takes no vote"
    click_on "Send Message"

    assert_message_text "/poll takes no vote", wait: 10
    assert_field "Write a message", with: ""
  end

  test "a command registered after page load still runs" do
    fill_in_markdown "message_markdown_source", with: "/deploy staging"
    @room.memberships.grant_to users(:bender)
    AgentSlashCommand.create!(agent: agents(:bender_agent), room: @room, name: "deploy")
    click_on "Send Message"

    assert_selector "#composer .composer__feedback", text: "Sent to Bender Bot", visible: true
    assert_field "Write a message", with: ""
    assert_no_selector ".message__body", text: "deploy staging"
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

  private
    def record_slash_command_fetches
      page.execute_script <<~JS
        window.__slashCommandFetchUrls = [];
        const originalFetch = window.fetch;
        window.fetch = (input, init) => {
          const url = typeof input === "string" ? input : input.url;
          if (url.includes("/autocompletable/slash_commands")) window.__slashCommandFetchUrls.push(url);
          return originalFetch(input, init);
        };
      JS
    end
end
