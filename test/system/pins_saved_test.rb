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

  test "pin notes render as compact notes with no message menu" do
    MessagePin.pin!(message: messages(:third), pinner: users(:david))
    note = Message.order(:id).last

    visit room_url(rooms(:designers))

    assert_selector "##{dom_id(note)}.message--system-note[role='note']",
      text: "David", wait: 10
    within_message note do
      assert_text "pinned a message"
      assert_selector ".message__system-note-icon"
      assert_no_selector ".message__avatar"
      assert_no_selector ".message__toolbar"
    end

    within_message note do
      find(".message__system-note-author").right_click
    end
    assert_no_selector "[data-message-actions-target='menu']", visible: true
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

  test "tomorrow at 9am resolves across the DST fall-back" do
    # The browser clock and time zone are fixed: noon in New York on
    # November 1, 2025, the day before clocks fall back. Tomorrow at 9am
    # is then 9am EST (UTC-5), an hour further from UTC than "now".
    browser_zone = page.evaluate_script("Intl.DateTimeFormat().resolvedOptions().timeZone")
    page.driver.browser.execute_cdp("Emulation.setTimezoneOverride", timezoneId: "America/New_York")

    begin
      travel_to Time.utc(2025, 11, 1, 16, 0) do
        visit room_url(rooms(:designers))

        open_message_menu messages(:third)
        click_on "Save for later", exact: true

        within ".message-save-dialog" do
          choose "Tomorrow at 9am (your time)"
          freeze_browser_clock_at "2025-11-01T12:00:00-04:00"
          click_on "Save"
        end

        assert_no_selector ".message-save-dialog[open]"

        assert_equal Time.utc(2025, 11, 2, 14, 0), SavedItem.sole.remind_at
      end
    ensure
      page.driver.browser.execute_cdp("Emulation.setTimezoneOverride", timezoneId: browser_zone)
    end
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

  private
    # Freezes `new Date()` and `Date.now()` in the current page at the
    # given instant, leaving constructed dates, parsing, and timers
    # untouched. Scoped to this document: the next visit starts clean.
    def freeze_browser_clock_at(iso_instant)
      page.execute_script(<<~JS)
        (() => {
          const RealDate = window.Date
          const frozenTime = new RealDate(#{iso_instant.to_json}).getTime()
          function FrozenDate(...args) {
            if (new.target) {
              return args.length === 0 ? new RealDate(frozenTime) : new RealDate(...args)
            }
            return new RealDate(frozenTime).toString()
          }
          FrozenDate.prototype = RealDate.prototype
          FrozenDate.now = () => frozenTime
          FrozenDate.parse = RealDate.parse.bind(RealDate)
          FrozenDate.UTC = RealDate.UTC.bind(RealDate)
          window.Date = FrozenDate
        })()
      JS
    end
end
