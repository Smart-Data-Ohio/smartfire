require "application_system_test_case"

class RoomHeaderTest < ApplicationSystemTestCase
  setup do
    @environment_names = Huddle::REQUIRED_ENVIRONMENT
    @original_livekit_environment = ENV.values_at(*@environment_names)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"
  end

  teardown do
    @environment_names.zip(@original_livekit_environment).each { |name, value| ENV[name] = value }
  end

  test "phone header keeps members, call, search and More with the rest in the menu" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      [ 360, 390, 500 ].each do |width|
        page.current_window.resize_to(width, 800)

        # Only the listed actions plus More stay in the bar.
        assert_selector "#header-overflow-button", visible: true
        assert_selector ".workspace-navigation__open", visible: true
        assert_selector ".room-header__name", visible: true
        assert_visible_nav_actions "Show members", "Join huddle"
        assert_search_visible
        assert_hidden_nav_actions "Show threads", "Show events", "Show pinned messages", "Show files"
        assert_no_selector "#nav .help-menu", visible: true
        assert_no_selector ".room-header__actions [aria-label^='Quick switcher']", visible: true
        assert_hidden_trailing_nav_actions

        # The menu holds the rest as labeled items.
        click_button "More actions"
        assert_selector "#header-overflow-menu", visible: true
        within "#header-overflow-menu" do
          assert_selector "[role='menuitem']", text: "Threads", visible: true
          assert_selector "[role='menuitem']", text: "Events", visible: true
          assert_selector "[role='menuitem']", text: "Pins", visible: true
          assert_selector "[role='menuitem']", text: "Files", visible: true
          assert_selector "[role='menuitem']", text: "Notifications", visible: true
          assert_selector "[role='menuitem']", text: "Room settings", visible: true
          assert_selector "[role='menuitem']", text: "Keyboard shortcuts", visible: true
          assert_selector "[role='menuitem']", text: "Restart tour", visible: true
          assert_selector "[role='menuitem']", text: "Quick switcher", visible: true
        end
        page.send_keys :escape
        assert_no_selector "#header-overflow-menu", visible: true

        assert_no_inner_scroll
        assert_no_horizontal_overflow
        assert_header_inside_viewport
      end

      # At 320px the capped voice stack stays instead of stepping aside.
      page.current_window.resize_to(320, 740)
      assert_selector "#header-overflow-button", visible: true
      assert_visible_nav_actions "Show members", "Join huddle"
      assert_search_visible
      assert_no_horizontal_overflow
      assert_header_inside_viewport
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "tablet header keeps notifications and settings visible with the rest in the menu" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      [ 700, 768, 1024 ].each do |width|
        page.current_window.resize_to(width, 800)

        assert_selector "#header-overflow-button", visible: true
        assert_visible_nav_actions "Show members", "Join huddle"
        assert_search_visible
        assert_hidden_nav_actions "Show threads", "Show events", "Show pinned messages", "Show files"
        assert_no_selector "#nav .help-menu", visible: true
        assert_no_selector ".room-header__actions [aria-label^='Quick switcher']", visible: true
        assert_visible_trailing_nav_actions

        click_button "More actions"
        assert_selector "#header-overflow-menu", visible: true
        within "#header-overflow-menu" do
          assert_selector "[role='menuitem']", text: "Threads", visible: true
          assert_selector "[role='menuitem']", text: "Events", visible: true
          assert_selector "[role='menuitem']", text: "Pins", visible: true
          assert_selector "[role='menuitem']", text: "Files", visible: true
          assert_selector "[role='menuitem']", text: "Keyboard shortcuts", visible: true
          assert_selector "[role='menuitem']", text: "Restart tour", visible: true
          assert_selector "[role='menuitem']", text: "Quick switcher", visible: true
          # Notifications, settings and stage stay in the bar on tablets.
          assert_no_selector "[role='menuitem']", text: "Notifications", visible: true
          assert_no_selector "[role='menuitem']", text: "Room settings", visible: true
        end
        page.send_keys :escape

        assert_no_inner_scroll
        assert_no_horizontal_overflow
        assert_header_inside_viewport
      end
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "desktop header is unchanged with no More button" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      [ 1280, 1400 ].each do |width|
        page.current_window.resize_to(width, 900)

        assert_no_selector "#header-overflow-button", visible: true
        assert_no_selector "#header-overflow-menu", visible: true
        assert_selector "#help-menu-button", visible: true
        # The member panel auto-opens on desktop, so its toggle reads Hide.
        assert_selector "body.member-panel-open"
        assert_visible_nav_actions "Hide members", "Join huddle"
        assert_search_visible
        assert_selector ".room-header__actions [data-thread-panel-target='browserToggle']", visible: true
        assert_visible_nav_actions "Show events", "Show pinned messages", "Show files"
        assert_selector ".room-header__actions [aria-label^='Quick switcher']", visible: true
        assert_visible_trailing_nav_actions

        assert_no_horizontal_overflow
        assert_header_inside_viewport
      end
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "every phone overflow menu item works" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      page.current_window.resize_to(390, 844)

      # Threads opens the thread panel.
      click_button "More actions"
      within "#header-overflow-menu" do
        find("[role='menuitem']", text: "Threads").click
      end
      assert_selector "body.thread-panel-open", wait: 5
      find(".thread-panel__close").click
      assert_no_selector "body.thread-panel-open", wait: 5

      # Pins opens the pins panel with its count badge.
      MessagePin.pin!(message: messages(:third), pinner: users(:jz))
      visit room_url(rooms(:designers))
      page.current_window.resize_to(390, 844)
      click_button "More actions"
      within "#header-overflow-menu" do
        assert_selector "[data-header-overflow-target='pinsCount']", text: "1"
        find("[role='menuitem']", text: "Pins").click
      end
      within ".pins-panel" do
        assert_text "Third time's a charm."
      end
      find(".pins-panel__close").click
      assert_no_selector ".pins-panel[open]"

      # Files opens the files view.
      click_button "More actions"
      within "#header-overflow-menu" do
        click_link "Files"
      end
      assert_selector "#room-files-title", text: "Files", wait: 10
      assert_current_path %r{/rooms/\d+/files}, wait: 10
      join_room rooms(:designers)
      page.current_window.resize_to(390, 844)

      # Events opens the events view.
      click_button "More actions"
      within "#header-overflow-menu" do
        click_link "Events"
      end
      assert_text "Upcoming", wait: 10
      assert_current_path %r{/rooms/\d+/events}, wait: 10
      join_room rooms(:designers)
      page.current_window.resize_to(390, 844)

      # Room settings opens the edit page.
      click_button "More actions"
      within "#header-overflow-menu" do
        click_link "Room settings"
      end
      assert_current_path %r{/edit}, wait: 10
      join_room rooms(:designers)
      page.current_window.resize_to(390, 844)

      # Keyboard shortcuts opens the sheet.
      click_button "More actions"
      within "#header-overflow-menu" do
        click_on "Keyboard shortcuts"
      end
      assert_selector "#keyboard-shortcuts[open]", wait: 5
      click_button "Close shortcuts"
      assert_no_selector "#keyboard-shortcuts[open]"

      # Quick switcher opens from the menu.
      click_button "More actions"
      within "#header-overflow-menu" do
        click_on "Quick switcher"
      end
      assert_selector "#quick-switcher[open]", wait: 5
      page.send_keys :escape
      assert_no_selector "#quick-switcher[open]"

      # Notifications opens an explicit chooser naming the current level.
      # Choosing Muted sets exactly muted (no blind cycle toward
      # invisible), updates the label, and leaves the room in the sidebar.
      click_button "More actions"
      within "#header-overflow-menu" do
        click_on "Notifications: Everything"
        assert_selector "#header-overflow-notifications [role='menuitemradio'][aria-checked='true']", text: "Everything"
        find("#header-overflow-notifications [role='menuitemradio']", text: "Muted").click
        assert_selector "[data-header-overflow-target='notificationsLabel']", text: "Notifications: Muted"
      end
      assert_equal "muted", rooms(:designers).memberships.find_by!(user: users(:jz)).reload.involvement
      assert_selector "#user_sidebar a[data-room-id='#{rooms(:designers).id}']", visible: :all
      # The header bell refreshes to the new level through its frame.
      assert_selector ".room-header__actions .button_to_change_notifying button.muted", visible: :all, wait: 10
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "the overflow menu scrolls within a landscape phone viewport" do
    room = Rooms::Stage.create_for({ name: "Town Hall", creator: users(:david) }, users: [ users(:david), users(:jz) ])
    sign_in "jz@37signals.com"
    join_room room

    begin
      page.current_window.resize_to(640, 360)
      click_button "More actions"
      assert_selector "#header-overflow-menu", visible: true

      # A stage room on a phone holds the most items, including the
      # phone-only stage, notifications and settings entries.
      within "#header-overflow-menu" do
        assert_selector "[role='menuitem']", text: "Stage", visible: true
      end

      menu_bottom, viewport_height = page.evaluate_script(<<~JS)
        [ document.getElementById("header-overflow-menu").getBoundingClientRect().bottom, window.innerHeight ]
      JS
      assert_operator menu_bottom, :<=, viewport_height, "the menu runs past the viewport bottom"

      # Arrow keys reach the last item and scroll it into view.
      press_keys :end
      assert_focused "#header-overflow-menu [aria-label='Quick switcher']"
      assert page.evaluate_script(<<~JS), "the last menu item is not scrolled into view"
        (() => {
          const menu = document.getElementById("header-overflow-menu").getBoundingClientRect()
          const item = document.querySelector("#header-overflow-menu [aria-label='Quick switcher']").getBoundingClientRect()
          return item.top >= menu.top && item.bottom <= menu.bottom
        })()
      JS

      # The last item is clickable despite the short viewport.
      within "#header-overflow-menu" do
        find("[role='menuitem']", text: "Quick switcher").click
      end
      assert_selector "#quick-switcher[open]", wait: 5
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "opening threads from the menu returns focus to More on close" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      page.current_window.resize_to(390, 844)
      click_button "More actions"
      within "#header-overflow-menu" do
        find("[role='menuitem']", text: "Threads").click
      end
      assert_selector "body.thread-panel-open", wait: 5

      find(".thread-panel__close").click
      assert_no_selector "body.thread-panel-open", wait: 5
      assert_focused "#header-overflow-button"
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "the overflow menu is keyboard accessible and closes on outside tap" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      page.current_window.resize_to(390, 844)

      find("#header-overflow-button").send_keys :arrow_down
      assert_selector "#header-overflow-menu", visible: true
      assert_equal "true", find("#header-overflow-button")["aria-expanded"]
      assert_focused "#header-overflow-menu [role='menuitem']"

      press_keys :arrow_down
      assert_focused "#header-overflow-menu a[href*='/events']"

      press_keys :escape
      assert_no_selector "#header-overflow-menu", visible: true
      assert_focused "#header-overflow-button"
      assert_equal "false", find("#header-overflow-button")["aria-expanded"]

      click_button "More actions"
      assert_selector "#header-overflow-menu", visible: true
      find(".room-header__name").click
      assert_no_selector "#header-overflow-menu", visible: true
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "the overflow dot lights for unread threads, not pins, and the pins badge hides at zero" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      page.current_window.resize_to(390, 844)
      assert_selector "#header-overflow-button", visible: true
      assert_selector "#header-overflow-button .header-overflow__dot[hidden]", visible: :all

      # No pins: the menu badge hides instead of showing "0".
      click_button "More actions"
      within "#header-overflow-menu" do
        assert_selector "[data-header-overflow-target='pinsCount'][hidden]", visible: :all
      end
      page.send_keys :escape

      # Pins fill the badge but never light the dot.
      MessagePin.pin!(message: messages(:third), pinner: users(:jz))
      visit room_url(rooms(:designers))
      page.current_window.resize_to(390, 844)
      click_button "More actions"
      within "#header-overflow-menu" do
        assert_selector "[data-header-overflow-target='pinsCount']:not([hidden])", text: "1"
      end
      page.send_keys :escape
      assert_selector "#header-overflow-button .header-overflow__dot[hidden]", visible: :all

      # An unread thread lights the dot once the browser reports it.
      thread = ChannelThread.create!(room: rooms(:designers), creator: users(:kevin), name: "Dot check")
      ThreadMembership.join!(thread, users(:jz))
      Message.create!(room: rooms(:designers), thread: thread, creator: users(:kevin),
        markdown_source: "News for the dot.", client_message_id: "dot-check-1")

      click_button "More actions"
      within "#header-overflow-menu" do
        find("[role='menuitem']", text: "Threads").click
      end
      assert_selector "#thread-panel [data-thread-panel-target='browserList'] .thread-panel__thread-item[data-unread='true']",
        text: "Dot check", wait: 10
      find(".thread-panel__close").click
      assert_no_selector "body.thread-panel-open", wait: 5

      assert_selector "#header-overflow-button .header-overflow__dot:not([hidden])", visible: true, wait: 10
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "the tour restarts from the overflow menu on phones" do
    users(:jz).update!(tour_completed_at: nil)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      page.current_window.resize_to(390, 844)
      click_on "Skip tour"
      assert_no_selector "#tour .tour__card", visible: true

      assert_selector "#help-menu-button", visible: :hidden
      assert_selector "#header-overflow-button", visible: true
      click_button "More actions"
      within "#header-overflow-menu" do
        click_on "Restart tour"
      end

      assert_selector "#tour .tour__card", visible: true
      assert_selector ".tour__progress", text: "Step 1 of 5"

      # Steps 4 and 5 highlight the overflow button itself on phones,
      # since the switcher and help buttons live inside its menu.
      3.times { click_on "Next" }
      assert_selector ".tour__progress", text: "Step 4 of 5"
      assert_selector "#header-overflow-button.tour__target"
      click_on "Next"
      assert_selector ".tour__progress", text: "Step 5 of 5"
      assert_selector "#header-overflow-button.tour__target"
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  private
    # The global search sits beside the actions: a field where the header
    # has room, an icon button that expands it where it does not.
    def assert_search_visible
      assert_selector "#global-search-input, #global-search .global-search__toggle", visible: true
    end

    def assert_hidden_nav_actions(*labels)
      labels.each do |label|
        assert_no_selector ".room-header__actions [aria-label='#{label}']", visible: true
      end
    end

    def assert_visible_nav_actions(*labels)
      labels.each do |label|
        assert_selector ".room-header__actions [aria-label='#{label}']", visible: true
      end
    end

    # Settings and the notification bell ride at the row's trailing edge.
    # They stay visible on tablets and desktops and join the menu on phones.
    def assert_visible_trailing_nav_actions
      assert_selector ".room-header__actions a[href$='/edit']", visible: true
      assert_selector ".room-header__actions .button_to_change_notifying", visible: true
    end

    def assert_hidden_trailing_nav_actions
      assert_no_selector ".room-header__actions a[href$='/edit']", visible: true
      assert_no_selector ".room-header__actions .button_to_change_notifying", visible: true
    end

    # The actions row keeps an inner scroll as a backstop, but nothing in
    # the supported matrix may actually use it: every visible action must
    # fit without clipping.
    def assert_no_inner_scroll
      assert page.evaluate_script(<<~JS), "the header actions row scrolls internally"
        (() => {
          const row = document.querySelector(".room-header__actions")
          return row.scrollWidth - row.clientWidth <= 1
        })()
      JS
    end

    def assert_no_horizontal_overflow
      assert page.evaluate_script("document.documentElement.scrollWidth <= window.innerWidth"),
        "the workspace overflows the viewport horizontally"
    end

    def assert_header_inside_viewport
      box, viewport = page.evaluate_script(<<~JS)
        [ document.querySelector("#nav").getBoundingClientRect().toJSON(), window.innerWidth ]
      JS
      assert_operator box["left"], :>=, 0, "the room header starts outside the viewport"
      assert_operator box["right"], :<=, viewport, "the room header ends outside the viewport"
    end
end
