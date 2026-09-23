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
    assert_equal [ dom_id(messages(:third)) ], tabbables

    newest_id = tabbables.first
    page.execute_script("document.getElementById('#{newest_id}').focus()")
    assert_selector "##{newest_id}:focus"

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

  test "a stream replacing the focused message keeps focus and the tab stop on its replacement" do
    message = find("##{dom_id(messages(:second))}")
    page.execute_script("arguments[0].focus()", message)
    assert_selector "##{dom_id(messages(:second))}:focus"

    page.execute_script <<~JS, dom_id(messages(:second))
      const clone = document.getElementById(arguments[0]).cloneNode(true);
      clone.setAttribute("data-replaced", "true");
      const stream = `<turbo-stream action="replace" target="${arguments[0]}"><template>${clone.outerHTML}</template></turbo-stream>`;
      Turbo.renderStreamMessage(stream);
    JS

    # Wait for the replacement itself: the stream renders asynchronously,
    # and asserting focus first would pass on the not-yet-replaced node.
    assert_selector "##{dom_id(messages(:second))}[data-replaced='true']", wait: 10
    assert_focus_and_tab_stop_on messages(:second)
  end

  test "a direct DOM swap of the focused message keeps focus and the tab stop on its replacement" do
    message = find("##{dom_id(messages(:second))}")
    page.execute_script("arguments[0].focus()", message)
    assert_selector "##{dom_id(messages(:second))}:focus"

    # Outside Turbo (which preserves focus itself), the list controller
    # moves the tab stop and focus to the same-id replacement.
    page.execute_script <<~JS, dom_id(messages(:second))
      const node = document.getElementById(arguments[0]);
      const clone = node.cloneNode(true);
      clone.setAttribute("data-replaced", "true");
      node.replaceWith(clone);
    JS

    assert_selector "##{dom_id(messages(:second))}[data-replaced='true']"
    assert_focus_and_tab_stop_on messages(:second)
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

  test "a late composer autofocus does not steal focus from a message" do
    # The composer autofocuses on connect via a deferred tick; under load
    # that tick can fire after an early message focus. Reconnecting the
    # composer with focus already on a message must leave it there.
    message = find("##{dom_id(messages(:third))}")
    page.execute_script("arguments[0].focus()", message)
    assert_selector "##{dom_id(messages(:third))}:focus"

    page.execute_script(<<~JS)
      const composer = document.querySelector("[data-controller~='composer']");
      const parent = composer.parentNode;
      const next = composer.nextSibling;
      parent.removeChild(composer);
      parent.insertBefore(composer, next);
    JS

    # Let the Stimulus reconnect and its deferred autofocus tick fire.
    sleep 0.5
    assert_selector "##{dom_id(messages(:third))}:focus"
    assert_no_selector "#message_markdown_source:focus"
  end

  test "up arrow from an empty composer still edits my last message" do
    editor = find_field("Write a message")
    editor.click
    editor.send_keys :up

    assert_selector "[data-composer-target='contextLabel']", text: "Editing Message", wait: 10
  end

  test "up-arrow-to-edit shows an error when the actions endpoint fails" do
    page.execute_script <<~JS
      window.__origFetch = window.fetch;
      window.fetch = (input, init = {}) => {
        const url = typeof input === "string" ? input : input.url;
        if (url.includes("/actions")) return Promise.resolve(new Response("{}", { status: 500 }));
        return window.__origFetch(input, init);
      };
    JS

    editor = find_field("Write a message")
    editor.click
    editor.send_keys :up

    assert_selector ".flash--client[role='alert']", text: "temporarily unavailable", wait: 10
  ensure
    page.execute_script("window.fetch = window.__origFetch") if page
  end

  test "forward reuses the menu-open metadata request instead of fetching again" do
    gate_actions_requests

    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Forward"

    assert_equal 1, page.evaluate_script("window.__actionFetches"),
      "expected menu open and forward to share one metadata request"
    release_actions_requests
    assert_selector "dialog[open]", visible: true, wait: 10
  ensure
    restore_fetch if page
  end

  test "a menu opened while an action waits does not redirect the pending action" do
    gate_actions_requests

    within_message(messages(:second)) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Forward"

    within_message(messages(:third)) do
      right_click_message
    end
    assert_selector "##{dom_id(messages(:third))}[data-message-actions-open]"
    release_actions_requests

    assert_no_selector "dialog[open]"
    assert_selector "##{dom_id(messages(:third))}[data-message-actions-open]"
  ensure
    restore_fetch if page
  end

  test "the menu closes before Turbo caches the page" do
    within_message(messages(:third)) do
      right_click_message
    end
    assert_message_menu_open

    page.execute_script("document.dispatchEvent(new Event('turbo:before-cache'))")

    assert_no_selector ".message[data-message-actions-open]"
    assert_selector "##{dom_id(messages(:third))}[aria-expanded='false']", visible: false
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

    # The pending copy carries the text immediately; wait for the delivered
    # replacement (which carries the server message id) instead.
    assert_selector ".message[data-message-id] .message__body", text: "Announce me once", wait: 10
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

  test "the message-list top padding does not apply to search results" do
    rules = page.evaluate_script(<<~JS)
      (() => {
        const hits = [];
        const scan = (rules) => {
          for (const rule of rules) {
            if (rule.type === CSSRule.STYLE_RULE && rule.style.getPropertyValue("padding-block-start")) {
              hits.push([ rule.selectorText, rule.style.getPropertyValue("padding-block-start") ]);
            }
            if (rule.cssRules) scan(rule.cssRules);
          }
        };
        for (const sheet of document.styleSheets) {
          try { scan(sheet.cssRules) } catch { /* cross-origin sheet */ }
        }
        return hits.filter(([ selector ]) => selector.includes(".messages"));
      })()
    JS

    assert_not_empty rules, "expected a message-list top padding rule"
    assert_empty rules.select { |(selector, _)| selector == ".messages" },
      "expected no unscoped .messages top padding rule"
    assert rules.any? { |(selector, _)| selector.include?(":not(.searches__results)") },
      "expected the top padding rule to exclude search results"
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

  test "text fields stay at 16px on touch devices without changing the desktop look" do
    board = Rooms::Board.create_for({ name: "Launch", creator: users(:jz) }, users: [ users(:jz) ])
    post = ChannelThread.create!(room: board, creator: users(:jz), name: "Ship it", work_status: "planned")

    visit room_thread_path(board, post)
    dismiss_pwa_install_prompt
    find("summary", text: "Update work").click

    fields = [
      ".board-post__form input[name='thread[tags]']",
      ".board-post__form select[name='thread[work_status]']"
    ]
    fields.each { |field| assert_selector field, visible: true }

    # The desktop look keeps its small sizing; the override below only
    # applies to coarse pointers, and these small fields prove the test
    # would catch a missing override.
    fields.each do |field|
      assert_operator font_size(field), :<, 16, "expected #{field} to keep its desktop sizing"
    end

    page.driver.browser.execute_cdp("Emulation.setTouchEmulationEnabled", enabled: true, maxTouchPoints: 5)
    assert page.evaluate_script("matchMedia('(pointer: coarse)').matches"),
      "expected touch emulation to report a coarse pointer"

    fields.each do |field|
      assert_operator font_size(field), :>=, 16, "expected #{field} at 16px on touch devices"
    end

    # Worst case: bare fields inheriting a tiny size are covered by the
    # same global rule.
    page.execute_script <<~JS
      document.body.insertAdjacentHTML("beforeend",
        "<div id='coarse-probe' style='font-size: 10px'>" +
        "<input id='coarse-input' type='text' aria-label='probe input'>" +
        "<select id='coarse-select' aria-label='probe select'><option>probe</option></select>" +
        "<textarea id='coarse-area' aria-label='probe textarea'></textarea></div>")
    JS
    %w[ coarse-input coarse-select coarse-area ].each do |id|
      assert_operator font_size("##{id}"), :>=, 16, "expected ##{id} at 16px on touch devices"
    end
  ensure
    page.driver.browser.execute_cdp("Emulation.setTouchEmulationEnabled", enabled: false) if page
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

  test "flash persists its 5-second minimum under reduced motion" do
    page.driver.browser.execute_cdp("Emulation.setEmulatedMedia", features: [ { name: "prefers-reduced-motion", value: "reduce" } ])

    visit user_profile_url
    fill_in "user_bio", with: "Reduced motion flash check"
    click_button "Save changes"

    assert_selector ".flash", wait: 10
    duration = page.evaluate_script("document.querySelector('.flash__inner').getAnimations().map(animation => animation.effect.getTiming().duration)")
    assert_equal [ 5000 ], duration

    # Travel to 4 seconds in: wall-clock time passes but the removal timer
    # has not run out, so the flash stays.
    page.execute_script("document.querySelector('.flash__inner').getAnimations()[0].currentTime = 4000")
    sleep 0.3
    assert_selector ".flash"

    # Travel past the end: the animation finishes and removes the flash.
    page.execute_script("document.querySelector('.flash__inner').getAnimations()[0].currentTime = 5100")
    assert_no_selector ".flash", wait: 10
  ensure
    page.driver.browser.execute_cdp("Emulation.setEmulatedMedia", features: [ { name: "prefers-reduced-motion", value: "no-preference" } ]) if page
  end

  test "flash dismisses on demand under reduced motion" do
    page.driver.browser.execute_cdp("Emulation.setEmulatedMedia", features: [ { name: "prefers-reduced-motion", value: "reduce" } ])

    visit user_profile_url
    fill_in "user_bio", with: "Reduced motion dismiss check"
    click_button "Save changes"

    assert_selector ".flash", wait: 10
    find(".flash__dismiss").click
    assert_no_selector ".flash"
  ensure
    page.driver.browser.execute_cdp("Emulation.setEmulatedMedia", features: [ { name: "prefers-reduced-motion", value: "no-preference" } ]) if page
  end

  private
    def assert_focus_and_tab_stop_on(message)
      assert_selector "##{dom_id(message)}:focus", wait: 10
      assert_equal dom_id(message), page.evaluate_script("document.activeElement.id")
      assert_equal 0, page.evaluate_script("document.getElementById('#{dom_id(message)}').tabIndex")
      assert_equal(-1, page.evaluate_script("document.getElementById('#{dom_id(messages(:third))}').tabIndex"),
        "expected the tab stop to stay on the replacement, not jump to the newest message")
    end

    def font_size(selector)
      page.evaluate_script("parseFloat(getComputedStyle(document.querySelector(\"#{selector}\")).fontSize)")
    end

    # Holds actions-endpoint fetches behind a gate so tests can interleave
    # menu opens while a request is in flight. Counts every attempt.
    def gate_actions_requests
      page.execute_script <<~JS
        window.__actionFetches = 0;
        window.__actionGates = [];
        window.__origFetch = window.fetch;
        window.fetch = (input, init = {}) => {
          const url = typeof input === "string" ? input : input.url;
          if (url.includes("/actions")) {
            window.__actionFetches++;
            return new Promise(resolve => window.__actionGates.push(() => resolve(window.__origFetch(input, init))));
          }
          return window.__origFetch(input, init);
        };
      JS
    end

    def release_actions_requests
      page.execute_script("window.__actionGates.forEach(release => release()); window.__actionGates = []")
    end

    def restore_fetch
      release_actions_requests
      page.execute_script("window.fetch = window.__origFetch")
    end
end
