module SystemTestHelper
  # Fast authenticated path: the test-only route verifies the same
  # credentials and issues the same session row and cookie as the login
  # form, skipping only the form round-trips. Every test still drives a
  # real browser session from here on.
  def sign_in(email_address, password = "secret123456")
    visit sign_in_for_tests_path(email_address: email_address, password: password)
    assert_selector "a.btn", text: "Designers"
  end

  def wait_for_cable_connection
    assert_selector "turbo-cable-stream-source[connected]", count: 3, visible: false
  end

  def join_room(room)
    visit room_url(room)
    wait_for_cable_connection
    dismiss_pwa_install_prompt
  end

  def send_message(message)
    if page.has_field?("message_markdown_source", visible: true, wait: 0)
      fill_in_markdown "message_markdown_source", with: message
    else
      fill_in_rich_text_area "message_body", with: message
    end
    click_on "Send Message"
  end

  def fill_in_markdown(locator, with:)
    editor = find_field(locator)
    editor.click
    # Pasting multiline source must not simulate desktop Enter-to-send. The
    # keyboard system tests exercise physical Enter and Shift+Enter separately.
    page.execute_script <<~JS, editor, with
      const [editor, source] = arguments;
      editor.value = source;
      editor.dispatchEvent(new InputEvent('input', {
        bubbles: true, inputType: 'insertFromPaste', data: source
      }));
    JS
  end

  def within_message(message, &block)
    within "#" + dom_id(message), &block
  end

  def assert_message_text(text, **options)
    assert_selector ".message[data-message-id] .message__body", text: text, **options
  end

  def assert_room_read(room)
    assert_selector ".rooms a", class: "!unread", text: "#{room.name}", wait: 5
  end

  def assert_room_unread(room)
    assert_selector ".rooms a", class: "unread", text: "#{room.name}", wait: 5
  end

  # Right-clicks the message body in the current scope. Call it inside
  # within_message, then assert on the shared menu outside the scope.
  def right_click_message
    find("[data-message-edit-format], [data-reply-target='body']", match: :first).right_click
  end

  def assert_message_menu_open
    assert_selector "[data-message-actions-target='menu']", visible: true, wait: 10
    assert_selector ".message[data-message-actions-open]"
  end

  def reveal_message_actions
    right_click_message
    assert_message_menu_open
  end

  # Opens the shared per-page message menu for one message: right-clicks
  # the message body, then asserts on the menu outside the message scope.
  def open_message_menu(message)
    within_message(message) do
      right_click_message
    end
    assert_message_menu_open
  end

  # Re-emits a message's broadcasts until its room shows unread (or the
  # timeout lapses). Direct model creates fan out by hand in tests, and a
  # broadcast emitted before the browser's unread subscription confirms
  # is lost; re-emitting only repaints the same badge, so this merely
  # compensates that race.
  def broadcast_until_unread(message, room, timeout: 10)
    deadline = Time.now + timeout
    loop do
      message.broadcast_create
      begin
        Capybara.using_wait_time(1) { assert_room_unread room }
        return
      rescue Minitest::Assertion, Capybara::ElementNotFound
        raise if Time.now > deadline
      end
    end
  end

  # Sends keys at the browser level (Selenium actions), landing on
  # whatever holds focus. Unlike element send_keys, chords and plain
  # keys reach global handlers even when focus sits on the body.
  def press_keys(*keys)
    modifiers = %i[ control shift alt meta command ]
    action = page.driver.browser.action

    keys.each do |key|
      if modifiers.include?(key)
        action.key_down(key)
      else
        action.send_keys(key)
      end
    end
    keys.reverse_each do |key|
      action.key_up(key) if modifiers.include?(key)
    end
    action.perform
  end

  def dismiss_pwa_install_prompt
    # No view renders this dialog target anymore, so the check below only
    # ever passes when a regression reintroduces it. join_room calls this
    # after the cable connects, by which point any rendered dialog is
    # present; a zero wait keeps the dismissal without burning 5 s per room
    # visit on the miss path.
    if page.has_css?("[data-pwa-install-target~='dialog']", visible: :visible, wait: 0)
      click_on("Close")
    end
  end

  # Simulates a touch long-press on a node. Pass move_by to drag during the
  # hold, which must cancel the press instead of opening the menu.
  def long_press(node, move_by: nil, hold: 0.7)
    action = page.driver.browser.action
    touch = action.add_pointer_input(:touch, "message-touch")
    action.move_to(node.native, device: "message-touch")
    action.pointer_down(:left, device: "message-touch")
    action.move_by(*move_by, device: "message-touch") if move_by
    action.pause(device: touch, duration: hold)
    action.pointer_up(:left, device: "message-touch")
    action.perform
  end
end
