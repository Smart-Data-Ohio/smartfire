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

    3.times { input.send_keys :backspace }
    input.send_keys "zephyr", :enter
    assert_current_path searches_path(q: "zephyr"), wait: 10
    assert_field "global-search-input", with: "zephyr"
    assert_selector "#search-results .message", text: "The zephyr rendezvous is at noon"
    assert users(:jz).searches.exists?(query: "zephyr")
    assert_selector "#search-title", text: "zephyr"
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

  test "desktop room headers keep the field visible beside the actions" do
    join_room @room

    begin
      [ 1400, 1920 ].each do |width|
        page.current_window.resize_to(width, 900)
        assert_selector "#global-search-input", visible: true
        assert_no_selector "#global-search .global-search__toggle", visible: true
        assert_no_horizontal_overflow
        assert_field_inside_viewport
      end
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  private
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
