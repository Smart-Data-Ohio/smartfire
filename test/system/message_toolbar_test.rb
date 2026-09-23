require "application_system_test_case"

class MessageToolbarTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "the toolbar stays hidden until hover or focus and labels every action" do
    assert_no_selector ".message__toolbar", visible: true

    within_message(messages(:third)) do
      find("[data-reply-target='body']").hover
      assert_selector ".message__toolbar", visible: true, wait: 10
      assert_selector "button[aria-label='React with thumbs up']", visible: true
      assert_selector "button[aria-label='Add reaction']", visible: true
      assert_selector "button[aria-label='Reply to message']", visible: true
      assert_selector "button[aria-label='Open thread']", visible: true
      assert_selector "button[aria-label='More message actions'][aria-haspopup='menu']", visible: true
    end
  end

  test "quick-react creates a boost from the toolbar" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "React with thumbs up" }

    assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "1", wait: 10
    assert_selector ".reaction-chip[data-reaction='👍'].reaction-chip--active"
  end

  test "reply and thread buttons drive the composer and the thread panel" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Reply to message" }
    assert_selector "[data-composer-target='contextLabel']", text: /Replying to JZ/, wait: 10
    click_button "Cancel message context"

    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Open thread" }
    assert_selector "#thread-panel [data-thread-panel-target='create']", visible: true, wait: 10
  end

  test "the more button opens the shared menu for its message" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "More message actions" }

    assert_message_menu_open
    assert_selector "##{dom_id(messages(:third))}[data-message-actions-open]"
    assert_selector "##{dom_id(messages(:third))} button[aria-label='More message actions'][aria-expanded='true']", visible: false

    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"
  end

  test "keyboard users reach the toolbar from a focused message" do
    message = find("##{dom_id(messages(:third))}")
    page.execute_script("arguments[0].focus()", message)

    page.send_keys :tab, :tab
    assert_equal "React with thumbs up",
      page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    page.send_keys :enter
    assert_selector ".reaction-chip[data-reaction='👍'] .reaction-chip__count", text: "1", wait: 10
  end

  test "the emoji picker searches and reacts" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }

    assert_selector "#emoji-picker-panel", visible: true, wait: 10
    assert_selector "#emoji-picker-tab-smileys[aria-selected='true']"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']", visible: true, wait: 10
    assert_equal "Search emoji and icons",
      page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    fill_in "Search emoji and icons", with: "fire"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Fire']", visible: true, wait: 10
    find("#emoji-picker-panel .emoji-picker__option[aria-label='Fire']").click

    assert_no_selector "#emoji-picker-panel", visible: true
    assert_selector ".reaction-chip[data-reaction='🔥'] .reaction-chip__count", text: "1", wait: 10
    assert Boost.exists?(content: "🔥", message: messages(:third))
  end

  test "the picker shows category tabs and switches between them" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel", visible: true, wait: 10

    assert_selector "#emoji-picker-panel [role='tab']", count: 11
    assert_selector "#emoji-picker-tab-recent:not([aria-selected='true'])"
    find("#emoji-picker-tab-people").click

    assert_selector "#emoji-picker-tab-people[aria-selected='true']"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Waving hand']", visible: true
    assert_no_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']"

    find("#emoji-picker-tab-flags").click
    assert_selector "#emoji-picker-tab-flags[aria-selected='true']"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Chequered flag']", visible: true, wait: 10
  end

  test "the picker loads its emoji data only on first open" do
    before = page.evaluate_script("performance.getEntriesByType('resource').map(entry => entry.name)")
    assert_not before.any? { |name| name.end_with?(".json") && name.include?("emoji") },
      "expected no emoji data fetch before the picker opens"

    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']", visible: true, wait: 10

    after = page.evaluate_script("performance.getEntriesByType('resource').map(entry => entry.name)")
    assert after.any? { |name| name.end_with?(".json") && name.include?("emoji") },
      "expected the emoji data fetch on first open"
  end

  test "the picker remembers recent reactions" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']", visible: true, wait: 10
    find("#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']").click
    assert_selector "##{dom_id(messages(:third))} .reaction-chip[data-reaction='😀'] .reaction-chip__count", text: "1", wait: 10
    assert Boost.exists?(content: "😀", message: messages(:third))

    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel", visible: true, wait: 10
    find("#emoji-picker-tab-recent").click

    assert_selector "#emoji-picker-tab-recent[aria-selected='true']"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']", visible: true
  end

  test "the picker Custom tab reacts with a workspace icon" do
    create_workspace_icon(name: "acme", title: "Acme Corp")

    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel", visible: true, wait: 10
    find("#emoji-picker-tab-custom").click

    assert_selector "#emoji-picker-tab-custom[aria-selected='true']"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Acme Corp'] img", visible: true, wait: 10
    find("#emoji-picker-panel .emoji-picker__option[aria-label='Acme Corp']").click

    assert_no_selector "#emoji-picker-panel", visible: true
    assert_selector "##{dom_id(messages(:third))} .reaction-chip[data-reaction=':acme:'] .reaction-chip__count", text: "1", wait: 10
    assert Boost.exists?(content: ":acme:", message: messages(:third))
  end

  test "the picker reacts with a brand icon shortcode" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel", visible: true, wait: 10

    fill_in "Search emoji and icons", with: "openai"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='OpenAI'] img", visible: true, wait: 10
    find("#emoji-picker-panel .emoji-picker__option[aria-label='OpenAI']").click

    assert_no_selector "#emoji-picker-panel", visible: true
    assert_selector "##{dom_id(messages(:third))} .reaction-chip[data-reaction=':openai:'] .reaction-chip__count", text: "1", wait: 10
    assert Boost.exists?(content: ":openai:", message: messages(:third))
  end

  test "picker arrows move through options, Enter selects, and Escape returns focus" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel", visible: true, wait: 10
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']", visible: true, wait: 10

    find_field("Search emoji and icons").send_keys :down
    assert_equal "Grinning face", page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    page.send_keys :right
    assert_equal "Grinning face with big eyes", page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    page.send_keys :escape
    assert_no_selector "#emoji-picker-panel", visible: true
    assert_equal "Add reaction", page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']", visible: true, wait: 10

    find_field("Search emoji and icons").send_keys :down
    page.send_keys :enter

    assert_no_selector "#emoji-picker-panel", visible: true
    assert_selector "##{dom_id(messages(:third))} .reaction-chip[data-reaction='😀'] .reaction-chip__count", text: "1", wait: 10
    assert Boost.exists?(content: "😀", message: messages(:third))
  end

  test "picker tabs move with arrow keys and switch the grid" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Grinning face']", visible: true, wait: 10

    find("#emoji-picker-tab-smileys").click
    page.send_keys :right

    assert_selector "#emoji-picker-tab-people[aria-selected='true']"
    assert_equal "emoji-picker-tab-people", page.evaluate_script("document.activeElement.id")
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Waving hand']", visible: true

    page.send_keys :left
    assert_selector "#emoji-picker-tab-smileys[aria-selected='true']"
    assert_equal "emoji-picker-tab-smileys", page.evaluate_script("document.activeElement.id")
  end

  private
    def hover_toolbar(message)
      within_message(message) do
        toolbar = find(".message__toolbar", visible: false)
        find("[data-reply-target='body']").hover
        assert_selector ".message__toolbar", visible: true, wait: 10
        toolbar
      end
    end
end
