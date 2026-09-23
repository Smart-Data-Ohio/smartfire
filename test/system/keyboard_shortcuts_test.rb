require "application_system_test_case"

class KeyboardShortcutsTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
  end

  test "? opens the shortcut sheet everywhere except while typing" do
    join_room rooms(:hq)

    find("body").send_keys("?")
    assert_selector "#keyboard-shortcuts[open]", wait: 5
    within "#keyboard-shortcuts" do
      assert_text "Quick switcher"
      assert_text "Previous / next room"
      assert_text "Previous / next unread room"
      assert_text "Mark current room read"
      assert_text "Message menu"
    end

    find("#keyboard-shortcuts").send_keys(:escape)
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

    find("body").send_keys(:alt, :arrow_up)
    assert_selector ".room-header__name", text: "Designers", wait: 10

    find("body").send_keys(:alt, :arrow_down)
    assert_selector ".room-header__name", text: "HQ", wait: 10
  end

  test "alt+shift+arrows jump between unread rooms" do
    designers = rooms(:designers)
    join_room rooms(:hq)

    designers.root_messages.create!(creator: users(:kevin), body: "Unread me", client_message_id: "unread-jump-1")
    assert_room_unread designers

    find("body").send_keys([ :alt, :shift, :arrow_down ])
    assert_selector ".room-header__name", text: "Designers", wait: 10
  end

  test "escape marks the current room read" do
    designers = rooms(:designers)
    join_room designers

    open_message_menu designers.root_messages.ordered.second
    click_on "Mark unread"
    assert_room_unread designers

    find("body").send_keys(:escape)
    assert_room_read designers
  end

  test "escape with a menu open closes the menu instead of marking read" do
    designers = rooms(:designers)
    join_room designers

    open_message_menu designers.root_messages.ordered.first
    click_on "Mark unread"
    assert_room_unread designers

    open_message_menu designers.root_messages.ordered.second
    find("[data-message-actions-target='menu']").send_keys(:escape)
    assert_no_selector ".message[data-message-actions-open]", wait: 5
    assert_room_unread designers
  end
end
