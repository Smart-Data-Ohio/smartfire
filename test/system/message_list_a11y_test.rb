require "application_system_test_case"

class MessageListA11yTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "the message list is a single tab stop with a roving tabindex" do
    tabbables = page.evaluate_script(<<~JS)
      Array.from(document.querySelectorAll("##{dom_id(@room, :messages)} > .message"))
        .filter(message => message.tabIndex === 0).map(message => message.id)
    JS
    assert_equal [ dom_id(@room.messages.ordered.first) ], tabbables

    first_id = tabbables.first
    page.execute_script("document.getElementById('#{first_id}').focus()")
    assert_selector "##{first_id}:focus"

    # The author's avatar link comes first in the message; the revealed
    # toolbar follows it.
    page.send_keys :tab
    assert page.evaluate_script("document.activeElement.classList.contains('avatar')"),
      "expected the first Tab stop inside a message to be the avatar link"

    page.send_keys :tab
    assert_equal "React with thumbs up",
      page.evaluate_script("document.activeElement.getAttribute('aria-label')"),
      "expected Tab from a focused message to reach its toolbar"
  end

  test "arrow keys move between messages" do
    messages = page.evaluate_script(<<~JS)
      Array.from(document.querySelectorAll("##{dom_id(@room, :messages)} > .message")).map(message => message.id)
    JS
    assert_operator messages.length, :>=, 2

    page.execute_script("document.getElementById('#{messages.first}').focus()")
    page.send_keys :down
    assert_selector "##{messages.second}:focus"

    page.send_keys :down
    assert_selector "##{messages.third}:focus" if messages.third

    page.send_keys :up
    assert_selector "##{messages.second}:focus"

    page.send_keys :home
    assert_selector "##{messages.first}:focus"

    page.send_keys :end
    assert_selector "##{messages.last}:focus"
  end

  test "the ContextMenu key opens the shared menu and Escape returns focus" do
    message = find("##{dom_id(messages(:third))}")
    page.execute_script("arguments[0].focus()", message)
    assert_selector "##{dom_id(messages(:third))}:focus"

    page.execute_script <<~JS, message
      arguments[0].dispatchEvent(new KeyboardEvent("keydown", {
        bubbles: true,
        cancelable: true,
        key: "ContextMenu"
      }))
    JS
    assert_message_menu_open

    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"
    assert_selector "##{dom_id(messages(:third))}:focus"
  end

  test "up arrow from an empty composer still edits my last message" do
    editor = find_field("Write a message")
    editor.click
    editor.send_keys :up

    assert_selector "[data-composer-target='contextLabel']", text: "Editing Message", wait: 10
  end

  test "the main message list is a live log" do
    list = find("##{dom_id(@room, :messages)}", visible: false)
    assert_equal "log", list["role"]
    assert_equal "polite", list["aria-live"]
    assert_equal "additions", list["aria-relevant"]
  end

  test "paginated history stays quiet past the insert, then the live region comes back" do
    count = Message::PAGE_SIZE + 5
    first_created_at = count.seconds.ago
    count.times do |index|
      Message.create!(
        room: @room,
        creator: users(:jz),
        markdown_source: "History post #{index}",
        client_message_id: "a11y-history-#{index}",
        created_at: first_created_at + index.seconds
      )
    end

    visit room_url(@room)
    wait_for_cable_connection
    dismiss_pwa_install_prompt
    assert_no_selector ".message", text: "History post 0"

    page.execute_script <<~JS
      window.__liveTimeline = [];
      window.__liveT0 = performance.now();
      const list = document.querySelector(".messages[role='log']");
      new MutationObserver(mutations => {
        for (const mutation of mutations) {
          const entry = { t: Math.round((performance.now() - window.__liveT0) * 10) / 10 };
          if (mutation.type === "attributes") {
            entry.live = list.getAttribute("aria-live");
            entry.busy = list.getAttribute("aria-busy");
          } else {
            entry.added = mutation.addedNodes.length;
          }
          window.__liveTimeline.push(entry);
        }
      }).observe(list, { attributes: true, attributeFilter: [ "aria-live", "aria-busy" ], childList: true });
    JS
    page.execute_script("document.querySelector('.messages').scrollTop = 0")

    assert_selector ".message", text: "History post 0", wait: 10
    assert_no_selector ".messages[aria-busy='true']", wait: 10

    timeline = page.evaluate_script("window.__liveTimeline")
    insert = timeline.find { |entry| entry["added"].to_i > 0 }
    assert_not_nil insert, "expected the timeline to record the history insert"

    assert timeline.any? { |entry| entry["live"] == "off" },
      "expected pagination to silence the live region while inserting"
    assert timeline.any? { |entry| entry["busy"] == "true" },
      "expected pagination to mark the list busy while inserting"

    restore = timeline.select { |entry| entry["live"] == "polite" && entry["t"] > insert["t"] }.first
    assert_not_nil restore, "expected the live region to be restored after pagination"
    assert_operator restore["t"] - insert["t"], :>=, 30,
      "expected the quiet state to still be in force after the insert"

    assert_equal "polite", page.evaluate_script("document.querySelector('.messages').getAttribute('aria-live')")
    assert_nil page.evaluate_script("document.querySelector('.messages').getAttribute('aria-busy')")
  end

  test "an edit replacement is not announced as an addition" do
    page.execute_script <<~JS
      window.__liveValues = [];
      new MutationObserver(mutations => {
        for (const mutation of mutations) window.__liveValues.push(mutation.target.getAttribute("aria-live"));
      }).observe(document.querySelector(".messages[role='log']"), { attributes: true, attributeFilter: [ "aria-live" ] });
    JS

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Edit message"
    assert_selector "[data-composer-target='contextLabel']", text: "Editing Message", wait: 10
    fill_in "Write a message", with: "Edited quietly"
    click_button "Send Message"

    assert_selector ".message__body", text: "Edited quietly", wait: 10
    assert_no_selector ".messages[aria-busy='true']", wait: 10
    assert_includes page.evaluate_script("window.__liveValues"), "off",
      "expected the edit replacement to silence the live region while rendering"
    assert_equal "polite", page.evaluate_script("document.querySelector('.messages').getAttribute('aria-live')")
  end

  test "an own message is not re-announced when its broadcast replaces the pending copy" do
    page.execute_script <<~JS
      window.__liveValues = [];
      new MutationObserver(mutations => {
        for (const mutation of mutations) window.__liveValues.push(mutation.target.getAttribute("aria-live"));
      }).observe(document.querySelector(".messages[role='log']"), { attributes: true, attributeFilter: [ "aria-live" ] });
    JS

    fill_in "Write a message", with: "Announce me once"
    click_button "Send Message"

    assert_selector ".message__body", text: "Announce me once", wait: 10
    assert_no_selector ".messages[aria-busy='true']", wait: 10
    assert_includes page.evaluate_script("window.__liveValues"), "off",
      "expected the pending-to-delivered replacement to render quietly"
    assert_equal "polite", page.evaluate_script("document.querySelector('.messages').getAttribute('aria-live')")
  end

  test "search results keep their menus and focusability" do
    result = @room.messages.create!(body: "A searchable menu result", creator: users(:jz), client_message_id: "menu-search-result")

    visit searches_url(q: "searchable")
    dismiss_pwa_install_prompt
    assert_selector "#search-results .message", text: "A searchable menu result"

    assert_selector "#search-results .message[tabindex='0'][aria-haspopup='menu']"

    within_message(result) do
      right_click_message
    end
    assert_message_menu_open
    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"

    long_press(find("#search-results ##{dom_id(result)}"))
    assert_message_menu_open
  end

  test "the standalone thread page keeps menus and focusability" do
    thread = ChannelThread.create!(room: @room, creator: users(:jz),
      name: "Menu audit thread", parent_message: messages(:third))
    reply = thread.messages.create!(room: @room, creator: users(:jz),
      markdown_source: "A thread reply with a menu", client_message_id: SecureRandom.uuid)

    visit room_thread_path(@room, thread)
    dismiss_pwa_install_prompt
    assert_selector "main.thread .message", count: 2
    assert_selector "main.thread .message[tabindex='0'][aria-haspopup='menu']"

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"

    within_message(reply) do
      right_click_message
    end
    assert_message_menu_open
    page.send_keys :escape
    assert_no_selector ".message[data-message-actions-open]"

    long_press(find("##{dom_id(reply)}"))
    assert_message_menu_open
  end

  test "the standalone message page keeps its menu and focusability" do
    visit room_message_path(@room, messages(:third))
    dismiss_pwa_install_prompt
    assert_selector ".message", text: "Third time's a charm."
    assert_selector ".message[tabindex='0'][aria-haspopup='menu']"

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
  end

  test "the viewport allows pinch zoom" do
    meta = find("meta[name='viewport']", visible: false)
    assert_equal "width=device-width, initial-scale=1, interactive-widget=resizes-content", meta["content"]
  end

  test "profile message and ban buttons have accessible names" do
    visit user_url(users(:kevin))
    assert_selector "button[aria-label='Message Kevin']"
    assert_selector "img[aria-label]", count: 0

    using_session("David") do
      sign_in "david@37signals.com"
      visit user_url(users(:kevin))
      assert_selector "button[aria-label='Message Kevin']"
      assert_selector "button", text: "Ban Kevin"
      assert_selector "img[aria-label]", count: 0
    end
  end

  test "flash persists under reduced motion and dismisses on demand" do
    page.driver.browser.execute_cdp("Emulation.setEmulatedMedia", features: [ { name: "prefers-reduced-motion", value: "reduce" } ])

    visit user_profile_url
    fill_in "user_bio", with: "Reduced motion flash check"
    click_button "Save changes"

    assert_selector ".flash", wait: 10
    assert_equal "5s", page.evaluate_script("getComputedStyle(document.querySelector('.flash__inner')).animationDuration")
    sleep 1
    assert_selector ".flash"

    find(".flash__dismiss").click
    assert_no_selector ".flash"
  ensure
    page.driver.browser.execute_cdp("Emulation.setEmulatedMedia", features: [ { name: "prefers-reduced-motion", value: "no-preference" } ]) if page
  end
end
