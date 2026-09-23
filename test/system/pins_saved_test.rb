require "application_system_test_case"

class PinsSavedTest < ApplicationSystemTestCase
  setup do
    sign_in "david@37signals.com"
    join_room rooms(:designers)
  end

  test "pinning from the menu badges the message and fills the panel, live for others" do
    using_session("Kevin") do
      sign_in "kevin@37signals.com"
      join_room rooms(:designers)
    end

    open_message_menu messages(:third)
    click_on "Pin message", exact: true

    within_message messages(:third) do
      assert_selector ".message__pin-badge", text: "Pinned"
    end
    assert_selector ".room-header__pins-count", text: "1"
    assert_text "pinned a message"

    using_session("Kevin") do
      within_message messages(:third) do
        assert_selector ".message__pin-badge", text: "Pinned", wait: BROADCAST_WAIT
      end
    end

    find("[aria-label='Show pinned messages']").click

    within ".pins-panel" do
      assert_text "Third time's a charm."
      assert_text "Pinned by"
      click_on "Jump to message"
    end

    assert_current_path %r{/rooms/\d+/@\d+}, wait: 10
    assert_text "Third time's a charm."
  end

  test "unpinning from the panel clears the badge everywhere" do
    MessagePin.pin!(message: messages(:third), pinner: users(:david))

    visit room_url(rooms(:designers))
    within_message messages(:third) do
      assert_selector ".message__pin-badge", text: "Pinned"
    end

    find("[aria-label='Show pinned messages']").click

    within ".pins-panel" do
      assert_text "Third time's a charm."
      click_on "Unpin"
      assert_text "No pinned messages yet"
    end

    find(".pins-panel__close").click

    within_message messages(:third) do
      assert_no_selector ".message__pin-badge", text: "Pinned"
    end
    assert_selector ".room-header__pins-count", text: "0"
  end

  test "saving with a reminder lists the message in Saved until done or removed" do
    open_message_menu messages(:third)
    click_on "Save for later", exact: true

    within ".message-save-dialog" do
      assert_text "Third time's a charm."
      choose "In 1 hour"
      click_on "Save"
    end

    # The save posts asynchronously; the dialog closes on success, so
    # waiting for it keeps the visit from cancelling the request.
    assert_no_selector ".message-save-dialog[open]"

    visit saved_items_url
    assert_selector "#saved-items-title", text: "Saved"
    assert_text "Third time's a charm."
    assert_text "In progress"
    assert_text "Reminds"

    click_on "Mark done"
    assert_text "Done"

    within("nav[aria-label='Saved filters']") { click_link "In progress" }
    assert_text "Nothing saved here yet."

    within("nav[aria-label='Saved filters']") { click_link "Done" }
    assert_text "Third time's a charm."

    click_on "Remove"
    assert_text "Nothing saved here yet."
  end

  test "saving with a custom reminder time" do
    open_message_menu messages(:second)
    click_on "Save for later", exact: true

    within ".message-save-dialog" do
      choose "Custom time"
      # Chrome's locale date editing mangles ISO keystrokes, so the
      # datetime-local value is set directly in canonical format.
      page.execute_script("document.getElementById('reminder_custom_at').value = '2030-06-01T09:00'")
      click_on "Save"
    end

    assert_no_selector ".message-save-dialog[open]"

    visit saved_items_url
    assert_text "Reminds"
  end
end
