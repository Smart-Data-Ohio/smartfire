require "application_system_test_case"

class GlobalSearchTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
    @room = rooms(:designers)
    users(:jz).searches.record("alpha")
    users(:jz).searches.find_by!(query: "alpha").update_column(:updated_at, 1.minute.ago)
    users(:jz).searches.record("beta")
  end

  test "the header field on a room page opens recents on focus and closes on Esc" do
    join_room @room

    input = find("#global-search-input[role='combobox']")
    assert_equal "false", input["aria-expanded"]
    assert_no_selector "#global-search-panel", visible: true

    input.click
    assert_selector "#global-search-panel", visible: true
    assert_equal "true", input["aria-expanded"]
    within "#global-search-listbox[role='listbox']" do
      assert_selector "[role='option']", count: 2
      # Most recent first.
      assert_selector "[role='option']:first-child", text: "beta"
    end

    input.send_keys :escape
    assert_no_selector "#global-search-panel", visible: true
    assert_equal "false", input["aria-expanded"]
    assert_focused "#global-search-input"
  end

  test "arrow keys move over recents and Enter opens the highlighted one" do
    join_room @room

    input = find("#global-search-input")
    input.click
    input.send_keys :arrow_down
    beta = find("[role='option']", text: "beta")
    assert_equal beta[:id], input["aria-activedescendant"]
    assert_equal "true", beta["aria-selected"]

    input.send_keys :arrow_down
    alpha = find("[role='option']", text: "alpha")
    assert_equal alpha[:id], input["aria-activedescendant"]
    assert_equal "false", beta["aria-selected"]

    input.send_keys :enter
    assert_current_path searches_path(q: "alpha"), wait: 10
    assert_field "global-search-input", with: "alpha"
  end

  test "typing filters recents and Enter submits the query to the results page" do
    @room.messages.create!(creator: users(:jz), markdown_source: "The zephyr rendezvous is at noon",
      client_message_id: "global-search-zephyr")
    join_room @room

    input = find("#global-search-input")
    input.click
    input.send_keys "alp"
    assert_selector "[role='option']", text: "alpha", visible: true
    assert_no_selector "[role='option']", text: "beta", visible: true

    assert_selector "#global-search-panel [role='status']", text: "1 recent search matches.", visible: :all

    input.send_keys "zzz"
    assert_selector ".global-search__empty", text: "No recent searches match.", visible: true
    assert_no_selector "#global-search-listbox", visible: true
    assert_selector "#global-search-panel [role='status']", text: "No recent searches match.", visible: :all

    6.times { input.send_keys :backspace }
    assert_no_selector ".global-search__empty", visible: true
    input.send_keys "zephyr", :enter
    assert_current_path searches_path(q: "zephyr"), wait: 10
    assert_field "global-search-input", with: "zephyr"
    assert_selector "#search-results .message", text: "The zephyr rendezvous is at noon"
    assert users(:jz).searches.exists?(query: "zephyr")
    assert_selector "#search-title", text: "zephyr"
    assert_selector "#nav .room-header__kind", text: /search/i
  end

  test "/ and Ctrl+Shift+F focus the header field" do
    join_room @room

    find(".room-header__name").click
    press_keys "/"
    assert_focused "#global-search-input"
    assert_selector "#global-search-panel", visible: true
    assert_equal "", find("#global-search-input").value

    press_keys :escape
    fill_in_markdown "message_markdown_source", with: ""
    find_field("message_markdown_source").send_keys("/")
    assert_focused "#message_markdown_source"

    find_field("message_markdown_source").send_keys(:control, :shift, "f")
    assert_focused "#global-search-input"
  end

  test "clearing from the header dropdown empties it in place" do
    join_room @room

    find("#global-search-input").click
    within "#global-search-panel" do
      assert_selector "[role='option']", count: 2
      accept_confirm { click_button "Clear recent searches" }
      assert_text "No recent searches yet.", wait: 10
      assert_no_selector "[role='option']"
    end

    assert_current_path room_path(@room)
    assert_equal 0, users(:jz).searches.count
  end

  test "the results page has no watermark, keeps the way back, and lists recents without a query" do
    visit searches_url

    assert_selector "#search-title", text: "Search", wait: 10
    # One "Search" in the header, not a kind label over a "Search" title.
    assert_no_selector "#nav .room-header__kind"
    assert_link "Back to Designers"
    within ".searches__recents" do
      assert_link "beta"
      assert_link "alpha"
    end
    assert_no_selector ".message-area--empty", visible: :all
    assert page.evaluate_script(<<~JS), "the results page shows a large icon"
      Array.from(document.querySelectorAll("#main-content img")).every(img => img.getBoundingClientRect().width <= 32)
    JS
    assert_no_selector "#footer form", visible: :all

    click_link "Back to Designers"
    assert_current_path room_path(@room), wait: 10
  end

  test "narrow headers fold the field into a button that expands it" do
    join_room @room

    begin
      page.current_window.resize_to(390, 844)

      assert_selector "#global-search .global-search__toggle", visible: true
      assert_no_selector "#global-search-input", visible: true
      assert_no_horizontal_overflow

      click_button "Search"
      assert_selector "#global-search-input", visible: true
      assert_focused "#global-search-input"
      assert_selector "#global-search-panel", visible: true
      assert_equal "true", find("#global-search .global-search__toggle", visible: :all)["aria-expanded"]
      assert_no_horizontal_overflow
      assert_field_inside_viewport

      press_keys :escape
      assert_no_selector "#global-search-input", visible: true
      assert_focused "#global-search .global-search__toggle"

      # The shortcut expands it too.
      find(".room-header__name").click
      press_keys "/"
      assert_focused "#global-search-input"
      find("#global-search-input").send_keys "charm", :enter
      assert_current_path searches_path(q: "charm"), wait: 10
      assert_no_horizontal_overflow
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "desktop room headers keep the field and every action visible" do
    join_room @room
    close_member_panel

    begin
      [ 1400, 1920 ].each do |width|
        page.current_window.resize_to(width, 900)
        assert_selector "#global-search-input", visible: true
        assert_no_selector "#global-search .global-search__toggle", visible: true
        assert_no_horizontal_overflow
        assert_field_inside_viewport
        assert page.evaluate_script(<<~JS), "the quick switcher button is blank at #{width}px"
          Array.from(document.querySelectorAll("#nav .room-header__action--switcher > *"))
            .some(child => child.getBoundingClientRect().width > 0)
        JS
      end
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "clearing from the results page drops cached pages so Back shows no stale recents" do
    join_room @room
    find("#global-search-input").click
    assert_selector "[role='option']", count: 2
    press_keys :escape

    page.execute_script("Turbo.visit('#{searches_path}')")
    assert_selector "#search-recents", text: "alpha", wait: 10
    within("#search-recents") { accept_confirm { click_button "Clear recent searches" } }
    assert_text "No recent searches yet.", wait: 10

    page.go_back
    assert_current_path room_path(@room), wait: 10
    find("#global-search-input").click
    within "#global-search-panel" do
      assert_text "No recent searches yet."
      assert_no_selector "[role='option']"
    end
  end

  test "the field keeps one trailing spot on every page" do
    page.current_window.resize_to(1400, 900)

    join_room @room
    close_member_panel
    room_offset = field_offset_from_header_end
    # Header padding, one gap and the help menu.
    assert_operator room_offset, :<, 96, "the field should sit at the trailing edge, before only the help menu"

    visit searches_url
    assert_selector "#search-title", wait: 10
    assert_in_delta room_offset, field_offset_from_header_end, 1

    visit user_profile_url
    assert_selector "#global-search-input", visible: true
    # A page without the workspace sidebar pads its header differently,
    # but the field is still the last thing before the help menu.
    assert_operator page.evaluate_script(<<~JS), :<=, 16
      document.querySelector("#nav > .help-menu").getBoundingClientRect().left - document.querySelector("#global-search-form").getBoundingClientRect().right
    JS
    field_left, logout_right = page.evaluate_script(<<~JS)
      [ document.querySelector("#global-search-form").getBoundingClientRect().left,
        document.querySelector("[data-action='sessions#logout:prevent']").getBoundingClientRect().right ]
    JS
    assert_operator logout_right, :<=, field_left, "the page's own controls sit before the field"
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "the room name keeps its full width beside the field on a desktop with the members panel open" do
    @room.update!(name: "Product engineering")
    page.current_window.resize_to(1440, 900)
    join_room @room
    open_member_panel

    assert_selector "#global-search-input", visible: true
    assert_no_horizontal_overflow
    assert page.evaluate_script(<<~JS), "the room name is truncated"
      (() => {
        const heading = document.querySelector("#nav .room-header__identity h1")
        const name = heading.querySelector(".room-header__name")
        return heading.scrollWidth <= heading.clientWidth && name.scrollWidth <= name.clientWidth
      })()
    JS
    assert_equal "Search", find("#global-search-input")["placeholder"]
    assert page.evaluate_script(<<~JS), "the placeholder is clipped"
      (() => {
        const input = document.querySelector("#global-search-input")
        const probe = document.createElement("span")
        probe.textContent = input.placeholder
        probe.style.cssText = `font: ${getComputedStyle(input).font}; position: absolute; visibility: hidden; white-space: pre`
        document.body.append(probe)
        const fits = probe.getBoundingClientRect().width <= input.clientWidth
        probe.remove()
        return fits
      })()
    JS
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  private
    def field_offset_from_header_end
      page.evaluate_script(<<~JS)
        document.querySelector("#nav").getBoundingClientRect().right - document.querySelector("#global-search-form").getBoundingClientRect().right
      JS
    end

    def member_panel_open?
      find(".member-panel-toggle")["aria-expanded"] == "true"
    end

    def open_member_panel
      find(".member-panel-toggle").click unless member_panel_open?
      assert_selector ".member-panel-toggle[aria-expanded='true']"
    end

    def close_member_panel
      find(".member-panel-toggle").click if member_panel_open?
      assert_selector ".member-panel-toggle[aria-expanded='false']"
    end

    def assert_no_horizontal_overflow
      assert page.evaluate_script("document.documentElement.scrollWidth <= window.innerWidth"),
        "the workspace overflows the viewport horizontally"
    end

    def assert_field_inside_viewport
      box, viewport = page.evaluate_script(<<~JS)
        [ document.querySelector("#global-search-form").getBoundingClientRect().toJSON(), window.innerWidth ]
      JS
      assert_operator box["left"], :>=, 0
      assert_operator box["right"], :<=, viewport
    end
end
