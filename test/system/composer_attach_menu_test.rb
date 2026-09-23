require "application_system_test_case"
require_relative "../support/drive_share_mocks"

class ComposerAttachMenuTest < ApplicationSystemTestCase
  include GoogleCalendarTestHelper
  include DriveShareMocks

  setup do
    WebMock.enable!
    WebMock.disable_net_connect!(allow_localhost: true)
  end

  teardown do
    WebMock.reset!
    WebMock.disable!
  end

  # Same belt-and-suspenders as the Drive system tests: WebMock must
  # never leak out of this file, even when a test or an earlier
  # teardown step errors, or later tests' chromedriver traffic breaks.
  def after_teardown
    super
  ensure
    WebMock.reset!
    WebMock.disable!
  end

  test "+ shows both attach options when Drive is available" do
    connect_google!(users(:jz), scopes: DRIVE_SCOPES)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    find("button.composer__attachment-btn").click

    assert_selector "button.composer__attachment-btn[aria-expanded='true']"
    assert_selector ".attach-menu [role='menuitem']", text: "From this device"
    assert_selector ".attach-menu [role='menuitem']", text: "From Google Drive"
  end

  test "From this device triggers the file input" do
    connect_google!(users(:jz), scopes: DRIVE_SCOPES)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)
    install_file_picker_recorder

    find("button.composer__attachment-btn").click
    click_button "From this device"

    assert_equal 1, file_picker_clicks
    assert_selector "button.composer__attachment-btn[aria-expanded='false']"
  end

  test "From Google Drive starts the legacy picker flow" do
    connect_google!(users(:jz), scopes: DRIVE_SCOPES)
    stub_google_drive_list
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    find("button.composer__attachment-btn").click
    click_button "From Google Drive"

    assert_selector ".drive-picker__item", text: "Q3 Planning"
  end

  test "From Google Drive starts the enhanced share flow when sharing is configured" do
    picker_env_before_test = [ ENV["GOOGLE_PICKER_API_KEY"], ENV["GOOGLE_CLOUD_PROJECT_NUMBER"] ]
    ENV["GOOGLE_PICKER_API_KEY"] = "test-picker-key"
    ENV["GOOGLE_CLOUD_PROJECT_NUMBER"] = "123456789012"

    sign_in "jz@37signals.com"
    join_room rooms(:designers)
    inject_drive_share_mocks(drive_scenario)

    find("button.composer__attachment-btn").click
    click_button "From Google Drive"

    within(".drive-share-dialog") do
      assert_selector ".drive-share-dialog__file", text: "Q3 Planning"
    end
  ensure
    ENV["GOOGLE_PICKER_API_KEY"], ENV["GOOGLE_CLOUD_PROJECT_NUMBER"] = picker_env_before_test
  end

  test "+ opens the file picker directly without Drive" do
    sign_in "kevin@37signals.com"
    join_room rooms(:designers)
    install_file_picker_recorder

    assert_no_selector ".attach-menu"

    find("button.composer__attachment-btn").click

    assert_equal 1, file_picker_clicks
  end

  test "arrow keys move between items and Escape closes back onto +" do
    connect_google!(users(:jz), scopes: DRIVE_SCOPES)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    page.execute_script("document.querySelector('button.composer__attachment-btn').focus()")
    assert_focused "button.composer__attachment-btn"

    page.send_keys :down

    assert_selector "button.composer__attachment-btn[aria-expanded='true']"
    assert_focused ".attach-menu [role='menuitem']:nth-child(1)"

    page.send_keys :down
    assert_focused ".attach-menu [role='menuitem']:nth-child(2)"

    page.send_keys :up
    assert_focused ".attach-menu [role='menuitem']:nth-child(1)"

    page.send_keys :escape
    assert_selector "button.composer__attachment-btn[aria-expanded='false']"
    assert_focused "button.composer__attachment-btn"
  end

  test "a tap outside closes the menu" do
    connect_google!(users(:jz), scopes: DRIVE_SCOPES)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    find("button.composer__attachment-btn").click
    assert_selector "button.composer__attachment-btn[aria-expanded='true']"

    find(".room-header__name").click

    assert_selector "button.composer__attachment-btn[aria-expanded='false']"
  end

  test "phone layout keeps the menu above the composer with no horizontal overflow" do
    connect_google!(users(:jz), scopes: DRIVE_SCOPES)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    page.current_window.resize_to(390, 844)
    find("button.composer__attachment-btn").click
    assert_selector ".attach-menu [role='menuitem']", text: "From Google Drive"

    geometry = page.evaluate_script(<<~JS)
      (() => {
        const menu = document.querySelector(".attach-menu").getBoundingClientRect();
        const button = document.querySelector("button.composer__attachment-btn").getBoundingClientRect();
        const items = Array.from(document.querySelectorAll(".attach-menu [role='menuitem']"))
          .map((item) => item.getBoundingClientRect().height);
        return {
          menu: { left: menu.left, right: menu.right, top: menu.top, bottom: menu.bottom },
          buttonTop: button.top,
          items,
          overflow: document.documentElement.scrollWidth > window.innerWidth + 1,
          viewportWidth: window.innerWidth
        };
      })()
    JS

    refute geometry["overflow"], "the page overflows horizontally with the menu open"
    assert_operator geometry["menu"]["left"], :>=, 0
    assert_operator geometry["menu"]["right"], :<=, geometry["viewportWidth"] + 1
    assert_operator geometry["menu"]["bottom"], :<=, geometry["buttonTop"] + 1,
      "expected the menu to sit above the + button"
    assert_not_empty geometry["items"]
    geometry["items"].each do |height|
      assert_operator height, :>=, 44, "expected menu items at least 44px tall"
    end
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "device files, paste, and drag-and-drop still preview uploads" do
    sign_in "kevin@37signals.com"
    join_room rooms(:designers)

    page.execute_script(<<~JS)
      const input = document.querySelector("#composer [data-attach-menu-target='fileInput']");
      const transfer = new DataTransfer();
      transfer.items.add(new File([ "hello" ], "hello.txt", { type: "text/plain" }));
      input.files = transfer.files;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    JS
    assert_selector "#composer .composer__file", text: "hello"

    page.execute_script(<<~JS)
      const editor = document.querySelector("#composer textarea");
      const transfer = new DataTransfer();
      transfer.items.add(new File([ "pasted" ], "pasted.png", { type: "image/png" }));
      editor.dispatchEvent(new ClipboardEvent("paste", { clipboardData: transfer, bubbles: true, cancelable: true }));
    JS
    assert_selector "#composer .composer__file", text: "pasted"

    page.execute_script(<<~JS)
      const transfer = new DataTransfer();
      transfer.items.add(new File([ "dropped" ], "dropped.txt", { type: "text/plain" }));
      const event = new DragEvent("drop", { bubbles: true, cancelable: true });
      Object.defineProperty(event, "dataTransfer", { value: transfer });
      document.querySelector("#composer").dispatchEvent(event);
    JS
    assert_selector "#composer .composer__file", text: "dropped"

    assert_selector "#composer .composer__file", count: 3
  end

  private
    def install_file_picker_recorder
      page.execute_script <<~JS
        window.filePickerClicks = 0;
        document.querySelector("#composer [data-attach-menu-target='fileInput']")
          .addEventListener("click", () => window.filePickerClicks++);
      JS
    end

    def file_picker_clicks
      page.evaluate_script("window.filePickerClicks")
    end
end
