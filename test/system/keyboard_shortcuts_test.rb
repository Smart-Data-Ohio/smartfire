require "application_system_test_case"
require "timeout"

class KeyboardShortcutsTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
  end

  test "? opens the shortcut sheet everywhere except while typing" do
    join_room rooms(:hq)

    # Printable characters only reach global handlers from a focusable
    # element under WebDriver, so ask from a sidebar row.
    row = find("#sidebar a[data-room-id='#{rooms(:hq).id}']")
    row.send_keys("?")
    assert_selector "#keyboard-shortcuts[open]", wait: 5
    within "#keyboard-shortcuts" do
      assert_text "Quick switcher"
      assert_text "Previous / next room"
      assert_text "Previous / next unread room"
      assert_text "Mark current room read"
      assert_text "Message menu"
    end

    press_keys(:escape)
    assert_no_selector "#keyboard-shortcuts[open]", wait: 5
  end

  test "? while typing stays in the composer" do
    join_room rooms(:hq)

    fill_in_markdown "message_markdown_source", with: ""
    find_field("message_markdown_source").send_keys("?")
    assert_no_selector "#keyboard-shortcuts[open]", wait: 2
    assert_equal "?", find_field("message_markdown_source").value
  end

  test "ctrl+k fires while typing and inserts no link markup" do
    join_room rooms(:hq)

    editor = find_field("message_markdown_source")
    editor.click
    editor.send_keys("hello")
    editor.send_keys(:control, "k")

    assert_selector "#quick-switcher[open]", wait: 5
    assert_equal "hello", editor.value
  end

  test "alt+up and alt+down move between rooms" do
    join_room rooms(:hq)

    # The composer autofocuses; Alt must fire from outside text inputs.
    find(".room-header__name").click
    press_keys(:alt, :arrow_up)
    assert_selector ".room-header__name", text: "Designers", wait: 10
    # Let the new room's composer autofocus land before moving focus off
    # it, or a late autofocus pulls focus back into the text field.
    assert_focused "#message_markdown_source", wait: 10

    find(".room-header__name").click
    press_keys(:alt, :arrow_down)
    assert_selector ".room-header__name", text: "HQ", wait: 10
  end

  test "alt+shift+arrows jump between unread rooms" do
    designers = rooms(:designers)
    join_room rooms(:hq)
    expire_connection users(:jz), designers

    message = designers.root_messages.create!(creator: users(:kevin), body: "Unread me", client_message_id: "unread-jump-1")
    broadcast_until_unread message, designers

    find(".room-header__name").click
    press_keys(:alt, :shift, :arrow_down)
    assert_selector ".room-header__name", text: "Designers", wait: 10
  end

  test "escape marks the current room read" do
    designers = rooms(:designers)
    join_room designers

    mark_current_room_unread designers.root_messages.ordered.second

    press_keys(:escape)
    assert_room_read designers
  end

  test "escape while typing leaves the room unread" do
    designers = rooms(:designers)
    join_room designers

    mark_current_room_unread designers.root_messages.ordered.first

    find_field("message_markdown_source").click
    press_keys(:escape)
    assert_room_unread designers
  end

  test "escape with a menu open closes the menu instead of marking read" do
    designers = rooms(:designers)
    join_room designers

    mark_current_room_unread designers.root_messages.ordered.first

    open_message_menu designers.root_messages.ordered.second
    press_keys(:escape)
    assert_no_selector ".message[data-message-actions-open]", wait: 5
    assert_room_unread designers
  end

  test "escape with the huddle theater open yields to the theater instead of marking read" do
    designers = rooms(:designers)
    join_room designers

    mark_current_room_unread designers.root_messages.ordered.first

    # Theater mode is a class on the huddle panel, not a dialog or
    # popover. The probe binds after page load the way the huddle
    # controller binds its own Escape handler when the theater opens:
    # if the global handler swallowed the event first, the probe
    # would see an already-prevented event and the room would go read.
    page.execute_script(<<~JS)
      let panel = document.getElementById("channel-huddle");
      if (!panel) {
        panel = document.createElement("aside");
        panel.id = "channel-huddle";
        document.body.appendChild(panel);
      }
      panel.classList.add("huddle--theater");
      window.__theaterEscapePrevented = "unseen";
      window.addEventListener("keydown", (event) => {
        if (event.key === "Escape") window.__theaterEscapePrevented = event.defaultPrevented;
      });
    JS

    press_keys(:escape)
    assert_equal false, page.evaluate_script("window.__theaterEscapePrevented")
    assert_room_unread designers

    page.execute_script(<<~JS)
      document.getElementById("channel-huddle").classList.remove("huddle--theater")
    JS

    press_keys(:escape)
    assert_room_read designers
  end

  test "escape in full screen leaves the room unread" do
    designers = rooms(:designers)
    join_room designers

    mark_current_room_unread designers.root_messages.ordered.first

    page.evaluate_script("document.documentElement.requestFullscreen()")
    assert page.evaluate_script("document.fullscreenElement !== null"), "expected full screen to engage"

    # Headless full screen resizes below the desktop breakpoint, which
    # closes the member drawer; wait for both to settle so the Escape
    # below meets nothing open but full screen itself. A synthetic
    # Escape: a trusted one would exit full screen in the browser
    # before any assertion could run.
    Timeout.timeout(10) do
      sleep 0.05 until page.evaluate_script("!matchMedia('(min-width: 80rem)').matches")
    end
    assert_no_selector "body.member-panel-open", wait: 5

    # A closed thread panel is not a modal, even in mobile layout.
    assert_no_selector ".thread-panel__surface[aria-modal='true']", visible: :all

    # The probe binds after page load, like the huddle's own Escape
    # handler, and observes synchronously: unlike the badge below, it
    # cannot pass before a buggy mark-read roundtrip lands.
    page.execute_script(<<~JS)
      window.__fullscreenEscapePrevented = "unseen";
      window.addEventListener("keydown", (event) => {
        if (event.key === "Escape") window.__fullscreenEscapePrevented = event.defaultPrevented;
      });
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    JS

    assert_equal false, page.evaluate_script("window.__fullscreenEscapePrevented")
    # The narrow fullscreen layout hides the sidebar rows, so match
    # the badge class regardless of visibility.
    assert_selector ".rooms a.unread", text: "Designers", visible: :all, wait: 5
  ensure
    page.evaluate_script("document.exitFullscreen().catch(() => {})") if page
  end

  test "alt+arrows while typing stay in the room" do
    join_room rooms(:hq)

    editor = find_field("message_markdown_source")
    editor.click
    editor.send_keys("hello")
    press_keys(:alt, :arrow_up)

    assert_selector ".room-header__name", text: "HQ", wait: 5
    assert_equal "hello", find_field("message_markdown_source").value

    press_keys(:alt, :shift, :arrow_down)
    assert_selector ".room-header__name", text: "HQ", wait: 5
  end

  test "ctrl+k ignores IME composition" do
    join_room rooms(:hq)

    page.evaluate_script(<<~JS)
      document.activeElement.dispatchEvent(new KeyboardEvent("keydown", {
        key: "k", ctrlKey: true, isComposing: true, bubbles: true, cancelable: true
      }))
    JS

    assert_no_selector "#quick-switcher[open]", wait: 2
  end

  test "ctrl+k does not open over an open modal dialog" do
    join_room rooms(:hq)

    row = find("#sidebar a[data-room-id='#{rooms(:hq).id}']")
    row.send_keys("?")
    assert_selector "#keyboard-shortcuts[open]", wait: 5

    press_keys(:control, "k")
    assert_no_selector "#quick-switcher[open]", wait: 2
    assert_selector "#keyboard-shortcuts[open]"

    press_keys(:escape)
    assert_no_selector "#keyboard-shortcuts[open]", wait: 5
  end

  test "ctrl+k still toggles the switcher closed" do
    join_room rooms(:hq)

    press_keys(:control, "k")
    assert_selector "#quick-switcher[open]", wait: 5

    press_keys(:control, "k")
    assert_no_selector "#quick-switcher[open]", wait: 5
  end

  private
    # The sign-in landing connects its room for 60 seconds, during which
    # the server skips it when marking unread. Expire that grace period
    # so the setup message below persists server-side.
    def expire_connection(user, room)
      user.memberships.find_by!(room: room).update_columns(connected_at: nil, connections: 0)
    end

    # Marks the room unread through the message menu and waits for the
    # roundtrip: the menu announces "Marked unread" once the DELETE
    # lands and the local badge echo is dispatched (the server
    # broadcast skips the current room by design, so only that echo
    # paints the badge).
    def mark_current_room_unread(message)
      wait_for_room_presence
      open_message_menu message
      click_on "Mark unread"
      assert_selector "[data-message-actions-target='status']", text: "Marked unread", visible: :all, wait: 10
      assert_room_unread message.room
    end

    # The room presence controller marks the current room read when its
    # subscription connects; under load that can land after the
    # mark-unread echo and wipe the badge, so wait for it first.
    def wait_for_room_presence
      Timeout.timeout(10) do
        sleep 0.05 until page.evaluate_script(<<~JS)
          !!window.Stimulus.getControllerForElementAndIdentifier(
            document.querySelector("[data-controller~='presence']"), "presence")?.channel
        JS
      end
    end
end
