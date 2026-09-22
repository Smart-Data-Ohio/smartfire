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
    assert_selector "#emoji-picker-panel .emoji-picker__option", count: EmojiHelper::REACTIONS.length, visible: true
    assert_equal "Search emoji and icons",
      page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    fill_in "Search emoji and icons", with: "fire"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='Fire']", visible: true, wait: 10
    find("#emoji-picker-panel .emoji-picker__option[aria-label='Fire']").click

    assert_no_selector "#emoji-picker-panel", visible: true
    assert Boost.exists?(content: "🔥", message: messages(:third))
    assert_selector ".reaction-chip[data-reaction='🔥'] .reaction-chip__count", text: "1", wait: 10
  end

  test "the picker reacts with a brand icon shortcode" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel", visible: true, wait: 10

    fill_in "Search emoji and icons", with: "openai"
    assert_selector "#emoji-picker-panel .emoji-picker__option[aria-label='OpenAI'] img", visible: true, wait: 10
    find("#emoji-picker-panel .emoji-picker__option[aria-label='OpenAI']").click

    assert_no_selector "#emoji-picker-panel", visible: true
    assert Boost.exists?(content: ":openai:", message: messages(:third))
    assert_selector "##{dom_id(messages(:third))} .boost-item", wait: 10
  end

  test "picker arrows move through options and Escape returns focus" do
    hover_toolbar(messages(:third))
    within_message(messages(:third)) { click_button "Add reaction" }
    assert_selector "#emoji-picker-panel", visible: true, wait: 10

    find_field("Search emoji and icons").send_keys :down
    assert_equal "Thumbs up", page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    page.send_keys :right
    assert_equal "Clapping", page.evaluate_script("document.activeElement.getAttribute('aria-label')")

    page.send_keys :escape
    assert_no_selector "#emoji-picker-panel", visible: true
    assert_equal "Add reaction", page.evaluate_script("document.activeElement.getAttribute('aria-label')")
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
