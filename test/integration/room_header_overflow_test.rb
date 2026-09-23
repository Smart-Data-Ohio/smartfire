require "test_helper"

class RoomHeaderOverflowTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "channel header overflow menu holds threads, events, pins, files, notifications, settings, help and switcher" do
    get room_url(rooms(:designers))
    assert_response :success

    assert_select "#header-overflow-button[aria-haspopup='menu'][aria-expanded='false'][aria-controls='header-overflow-menu']", count: 1
    assert_select "#header-overflow-menu[role='menu']", count: 1

    within_overflow_menu do
      assert_select "[role='menuitem']", text: /Threads/, count: 1
      assert_select "[role='menuitem']", text: /Events/, count: 1
      assert_select "[role='menuitem']", text: /Pins/, count: 1
      assert_select "[role='menuitem']", text: /Files/, count: 1
      assert_select "[role='menuitem'].header-overflow__item--phone-only", text: /Notifications/, count: 1
      assert_select "[role='menuitem'].header-overflow__item--phone-only", text: /Room settings/, count: 1
      assert_select "[role='menuitem']", text: /Keyboard shortcuts/, count: 1
      assert_select "[role='menuitem']", text: /Restart tour/, count: 1
      assert_select "[role='menuitem']", text: /Quick switcher/, count: 1
    end

    # The header buttons carry the hiding hooks the overflow CSS keys on.
    assert_select ".room-header__actions .room-header__action--threads", count: 1
    assert_select ".room-header__actions .room-header__action--events", count: 1
    assert_select ".room-header__actions .room-header__action--pins", count: 1
    assert_select ".room-header__actions .room-header__action--files", count: 1
    assert_select ".room-header__actions .room-header__action--settings", count: 1
    assert_select ".room-header__actions .button_to_change_notifying", count: 1

    # Threads shares its body-level toggle so badges update in place; pins
    # forwards to the scoped panel button.
    assert_select "#header-overflow-menu [data-action~='thread-panel#toggle'][data-thread-panel-target='browserToggle']", count: 1
    assert_select "#header-overflow-menu [data-header-overflow-forward-value=\".room-header__actions [aria-label='Show pinned messages']\"]", count: 1
  end

  test "stage rooms keep a stage toggle in the bar and a phone-only stage item in the menu" do
    room = Rooms::Stage.create_for({ name: "Town Hall", creator: users(:david) }, users: [ users(:david), users(:jz) ])

    get room_url(room)
    assert_response :success

    assert_select ".room-header__actions .room-header__action--stage[aria-label='Show stage']", count: 1
    within_overflow_menu do
      assert_select "[role='menuitem'].header-overflow__item--phone-only", text: /Stage/, count: 1
    end
  end

  test "direct rooms omit threads from the overflow menu" do
    get room_url(rooms(:david_and_jason))
    assert_response :success

    assert_select ".room-header__actions .room-header__action--threads", count: 0
    within_overflow_menu do
      assert_select "[role='menuitem']", text: /Threads/, count: 0
      assert_select "[role='menuitem']", text: /Events/, count: 1
      assert_select "[role='menuitem']", text: /Pins/, count: 1
      assert_select "[role='menuitem']", text: /Files/, count: 1
    end
  end

  test "board header overflow menu holds events, files and the shared items without threads, pins or stage" do
    room = Rooms::Board.create_for({ name: "Sprint", creator: users(:david) }, users: [ users(:david) ])

    get room_url(room)
    assert_response :success

    assert_select "#header-overflow-button", count: 1
    within_overflow_menu do
      assert_select "[role='menuitem']", text: /Events/, count: 1
      assert_select "[role='menuitem']", text: /Files/, count: 1
      assert_select "[role='menuitem']", text: /Threads/, count: 0
      assert_select "[role='menuitem']", text: /Pins/, count: 0
      assert_select "[role='menuitem']", text: /Stage/, count: 0
      assert_select "[role='menuitem']", text: /Quick switcher/, count: 1
    end
  end

  private
    def within_overflow_menu(&)
      within "#header-overflow-menu", &
    end

    def within(selector, &)
      assert_select selector do
        yield
      end
    end
end
