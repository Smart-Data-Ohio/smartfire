require "application_system_test_case"

class ComposerTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  # Regression guard: origin/main already destroyed the suggestion controller
  # on blur (the old `suggestionController.active` check was never a real
  # property, so it always took the destroy branch). This pins the behavior.
  test "blurring an open autocomplete does not leave a zombie that swallows Enter" do
    editor = find_field("Write a message")
    editor.set(":thu")
    assert_selector "suggestion-option", text: "thumbsup"

    page.execute_script("document.activeElement.blur()")
    assert_no_selector "suggestion-option"

    editor.click
    editor.send_keys("hello")
    editor.send_keys(:enter)

    assert_message_text "hello", wait: 10
    assert_field "Write a message", with: ""
  end

  test "a stale icon response does not poison the suggestion commit" do
    page.execute_script <<~JS
      const originalFetch = window.fetch;
      window.fetch = (input, init) => {
        const url = typeof input === "string" ? input : input.url;
        if (url.includes("/autocompletable/icons") && /[?&]q=zx(&|$)/.test(url)) {
          return new Promise(resolve => setTimeout(() => resolve(originalFetch(input, init)), 1500));
        }
        return originalFetch(input, init);
      };
    JS

    editor = find_field("Write a message")
    editor.set(":zx")
    sleep 0.6
    editor.set(":open")
    assert_selector "suggestion-option", text: "OpenAI"
    sleep 1.6
    editor.send_keys(:enter)

    assert_field "Write a message", with: ":openai: "
  end

  test "mention queries are URL-encoded" do
    page.execute_script <<~JS
      window.__composerTestUrls = [];
      const originalFetch = window.fetch;
      window.fetch = (input, init) => {
        const url = typeof input === "string" ? input : input.url;
        if (url.includes("/autocompletable/users")) window.__composerTestUrls.push(url);
        return originalFetch(input, init);
      };
    JS

    find_field("Write a message").set("@kev+in")
    urls = nil
    page.document.synchronize(Capybara.default_max_wait_time) do
      urls = page.evaluate_script("window.__composerTestUrls")
      raise Capybara::ElementNotFound if urls.empty?
    end

    assert urls.any? { |url| url.include?("query=kev%2Bin") }, "expected an encoded query in #{urls.inspect}"
  end

  test "composer autocomplete exposes combobox semantics over a polite listbox" do
    editor = find_field("Write a message")
    assert_equal "combobox", editor["role"]
    assert_equal "list", editor["aria-autocomplete"]
    assert_equal "false", editor["aria-expanded"]

    editor.set(":open")
    assert_selector "suggestion-option", text: "OpenAI"

    assert_equal "true", editor["aria-expanded"]
    list_id = editor["aria-controls"]
    assert list_id, "expected aria-controls while the list is open"
    assert_selector "suggestion-select##{list_id}[role='listbox'][aria-live='polite']"
    assert_selector "suggestion-select##{list_id} suggestion-option[role='option']"

    first_active = editor["aria-activedescendant"]
    assert first_active, "expected an activedescendant while the list is open"
    assert_selector "suggestion-option##{first_active}[selected]"

    editor.send_keys(:down)
    second_active = editor["aria-activedescendant"]
    assert second_active, "expected an activedescendant after moving"
    assert_not_equal first_active, second_active

    editor.send_keys(:escape)
    assert_equal "false", editor["aria-expanded"]
    assert_nil editor["aria-activedescendant"]
  end

  test "composing text does not commit a suggestion or send the message" do
    assert page.evaluate_script("new KeyboardEvent('x', { isComposing: true }).isComposing === true"),
      "expected the browser to support isComposing in KeyboardEventInit"

    editor = find_field("Write a message")
    editor.set(":open")
    assert_selector "suggestion-option", text: "OpenAI"

    page.execute_script <<~JS, editor
      arguments[0].dispatchEvent(new KeyboardEvent("keydown", {
        key: "Enter", keyCode: 13, bubbles: true, cancelable: true, isComposing: true
      }))
    JS

    assert_field "Write a message", with: ":open"
    assert_selector "suggestion-option", text: "OpenAI"
    sleep 0.5
    assert_not Message.exists?(markdown_source: ":open")

    editor.send_keys(:enter)
    assert_field "Write a message", with: ":openai: "
  end

  test "clicking a reply preview scrolls to the loaded message instead of navigating" do
    target = messages(:third)
    reply = send_reply(target, "A reply for preview click")

    within_message(reply) do
      find(".message__reply-preview-link").click
    end

    assert_selector "##{dom_id(target)}.message--reply-target", wait: 10
    assert_equal room_path(@room), current_path
  end

  test "clicking a reply preview falls back to the permalink when the target is not loaded" do
    target = messages(:third)
    reply = send_reply(target, "A reply for preview fallback")

    # Stand in for a target paginated out of the loaded history: the preview
    # can no longer scroll to it, so the click must follow the permalink.
    page.execute_script("document.getElementById(arguments[0]).remove()", dom_id(target))
    assert_no_selector "##{dom_id(target)}"

    page.execute_script <<~JS
      window.__composerTestUrls = [];
      document.addEventListener("turbo:before-fetch-request", event => {
        window.__composerTestUrls.push(String(event.detail.url));
      });
    JS

    within_message(reply) do
      find(".message__reply-preview-link").click
    end

    page.document.synchronize(Capybara.default_max_wait_time) do
      urls = page.evaluate_script("window.__composerTestUrls")
      raise Capybara::ElementNotFound unless urls.any? { |url| url.include?("/@") }
    end
    assert_no_selector ".message--reply-target"
  end

  test "deleting a replied-to message turns open reply previews into a tombstone" do
    target = messages(:third)
    reply = send_reply(target, "A reply whose source goes away")

    open_message_menu(target)
    accept_confirm { click_on "Delete message", exact: true }
    assert_no_selector "##{dom_id(target)}", wait: 10

    within_message(reply) do
      assert_selector ".message__reply-preview", text: "Replying to a deleted message", wait: 10
      assert_no_selector ".message__reply-preview-link"
    end
  end

  test "two typers with the same name do not merge" do
    users(:kevin).update!(name: "David")

    using_session("David") do
      sign_in "david@37signals.com"
      join_room @room
    end

    using_session("Kevin") do
      sign_in "kevin@37signals.com"
      join_room @room
    end

    # Typing expires after 5 seconds, and signing in takes longer than
    # that, so refresh both back to back before asserting them at once.
    using_session("David") do
      fill_in "Write a message", with: "hi from david"
    end
    using_session("Kevin") do
      fill_in "Write a message", with: "hi from kevin"
    end

    assert_selector "[data-typing-notifications-target='author']", exact_text: "David, David", wait: 10

    using_session("Kevin") do
      fill_in "Write a message", with: "hi from kevin"
    end
    using_session("David") do
      find_field("Write a message").set("")
    end

    assert_selector "[data-typing-notifications-target='author']", exact_text: "David", wait: 10

    using_session("Kevin") do
      find_field("Write a message").set("")
    end

    assert_no_selector ".typing-indicator--active", wait: 10
  end

  test "composer drafts persist per room and clear on send" do
    pets = rooms(:pets)
    pets.memberships.grant_to(users(:jz))

    fill_in "Write a message", with: "Designers draft"
    key = "campfire.composer.draft.#{users(:jz).id}.#{@room.id}.main"
    assert_equal "Designers draft", page.evaluate_script("window.localStorage.getItem(#{key.to_json})")

    join_room pets
    assert_field "Write a message", with: ""

    fill_in "Write a message", with: "Pets draft"

    join_room @room
    assert_field "Write a message", with: "Designers draft"

    join_room pets
    assert_field "Write a message", with: "Pets draft"
    click_on "Send Message"
    assert_message_text "Pets draft", wait: 10

    join_room @room
    assert_field "Write a message", with: "Designers draft"

    join_room pets
    assert_field "Write a message", with: ""
  end

  test "thread drafts persist per thread without touching the channel draft" do
    rooms(:pets).memberships.grant_to(users(:jz))
    thread_name = "Composer draft thread"
    open_threads
    click_button "New thread"
    assert_selector "#thread-panel [data-thread-panel-target='create']", visible: true, wait: 10
    fill_in "Thread name", with: thread_name
    fill_in "First message", with: "The thread for draft persistence."
    find("#thread-panel [data-thread-panel-target='createSubmit']").click
    assert_selector "#thread-panel [data-thread-panel-target='conversation']", visible: true, wait: 10

    within "#thread-panel" do
      fill_in "Write a thread reply", with: "Thread draft"
    end
    fill_in "Write a message", with: "Channel draft"

    join_room rooms(:pets)
    join_room @room

    assert_field "Write a message", with: "Channel draft"

    open_threads
    find("#thread-panel [data-thread-panel-target='browserList'] .thread-panel__thread-item", text: thread_name).click
    assert_selector "#thread-panel [data-thread-panel-target='conversation']", visible: true, wait: 10
    within "#thread-panel" do
      assert_field "Write a thread reply", with: "Thread draft"
      fill_in "Write a thread reply", with: "Thread draft sent"
      click_button "Send Reply"
      assert_selector ".thread-panel__thread-content .message__body", text: "Thread draft sent", wait: 10
    end

    join_room rooms(:pets)
    join_room @room
    open_threads
    find("#thread-panel [data-thread-panel-target='browserList'] .thread-panel__thread-item", text: thread_name).click
    assert_selector "#thread-panel [data-thread-panel-target='conversation']", visible: true, wait: 10
    within "#thread-panel" do
      assert_field "Write a thread reply", with: ""
    end
  end

  private
    def send_reply(target, source)
      open_message_menu(target)
      click_on "Reply", exact: true
      assert_selector "[data-composer-target='contextLabel']", text: /Replying to/, wait: 10
      fill_in_markdown "Write a message", with: source
      click_on "Send Message"
      assert_selector ".message__reply-preview", text: target.plain_text_body.truncate(30), wait: 10
      Message.find_by!(markdown_source: source)
    end

    def open_threads
      find("[data-thread-panel-target='browserToggle']").click unless page.has_css?("body.thread-panel-open", wait: 0)
      assert_selector "#thread-panel[aria-hidden='false']", visible: true, wait: 10
    end
end
