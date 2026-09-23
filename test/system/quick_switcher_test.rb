require "application_system_test_case"

# The Ctrl+K quick switcher, driven keyboard-only: open, filter, move,
# and open rooms, people and threads without touching the mouse.
class QuickSwitcherTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
  end

  test "keyboard-only room jump filters and opens with enter" do
    join_room rooms(:hq)

    open_switcher
    fill_in "Jump to a room, person, or thread", with: "Desig"
    assert_selector "#quick-switcher-listbox [role='option']", text: "Designers", wait: 5

    find_field("Jump to a room, person, or thread").send_keys(:enter)
    assert_selector ".room-header__name", text: "Designers", wait: 10
  end

  test "arrow keys move the active option" do
    join_room rooms(:hq)

    open_switcher
    fill_in "Jump to a room, person, or thread", with: "a"
    assert_selector "#quick-switcher-listbox [role='option'][aria-selected='true']", wait: 5

    titles = page.evaluate_script(<<~JS)
      Array.from(document.querySelectorAll("#quick-switcher-listbox [role='option'] .quick-switcher__title"))
        .map(node => node.textContent)
    JS
    assert titles.count >= 2, "expected several matches for 'a', got #{titles.inspect}"

    combobox.send_keys(:arrow_down)
    assert_active_title titles[1]

    combobox.send_keys(:arrow_up)
    assert_active_title titles[0]

    combobox.send_keys(:end)
    assert_active_title titles.last
  end

  test "escape closes the switcher" do
    join_room rooms(:hq)

    open_switcher
    combobox.send_keys(:escape)
    assert_no_selector "#quick-switcher[open]", wait: 5
  end

  test "empty query leads with recent rooms" do
    join_room rooms(:hq)
    join_room rooms(:designers)

    open_switcher
    within "#quick-switcher-listbox" do
      assert_text "Recent", wait: 5
      assert_text "HQ"
    end
  end

  test "choosing a person without a DM opens a new one" do
    join_room rooms(:hq)

    open_switcher
    fill_in "Jump to a room, person, or thread", with: "Kevin"
    assert_selector "#quick-switcher-listbox [role='option']", text: "Kevin", wait: 5

    assert_difference -> { Room.count } do
      find_field("Jump to a room, person, or thread").send_keys(:enter)
    end
    assert_selector ".room-header__name", text: "Kevin", wait: 10
  end

  private
    def open_switcher
      find("body").send_keys(:control, "k")
      assert_selector "#quick-switcher[open]", wait: 5
    end

    def combobox
      find_field("Jump to a room, person, or thread")
    end

    def assert_active_title(expected)
      active = page.evaluate_script(<<~JS)
        document.querySelector("#quick-switcher-listbox [aria-selected='true'] .quick-switcher__title")?.textContent
      JS
      assert_equal expected, active
    end
end
