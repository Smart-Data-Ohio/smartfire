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

  test "phone header keeps every visible action unclipped" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      # At 360px five actions remain (members, search, huddle, settings,
      # notifications); everything else steps aside to a reachable home.
      page.current_window.resize_to(360, 740)
      assert_hidden_nav_actions "Show files", "Show events", "Show pinned messages", "Show threads"
      assert_selector "#nav .help-menu", visible: :hidden
      assert_visible_nav_actions "Show members", "Search messages", "Join huddle"
      assert_trailing_nav_actions
      assert_no_inner_scroll
      assert_no_horizontal_overflow

      # At 390px the threads browser rejoins the row; the drawer flow the
      # threads tests drive must keep working on phones.
      page.current_window.resize_to(390, 844)
      assert_hidden_nav_actions "Show files", "Show events", "Show pinned messages"
      assert_visible_nav_actions "Show members", "Search messages", "Show threads", "Join huddle"
      assert_no_inner_scroll
      assert_no_horizontal_overflow

      # At 500px the same set holds with room to spare, with the member
      # panel open or closed (below 80rem it is an overlay).
      page.current_window.resize_to(500, 800)
      assert_visible_nav_actions "Show members", "Search messages", "Show threads", "Join huddle"
      assert_no_inner_scroll
      click_button "Show members"
      assert_selector "#channel-members", visible: true
      assert_no_inner_scroll
      assert_no_horizontal_overflow
      page.send_keys :escape
      assert_no_selector "#channel-members", visible: true
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  test "the tour restarts from the sidebar help menu on phones" do
    users(:jz).update!(tour_completed_at: nil)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    begin
      page.current_window.resize_to(390, 844)
      click_on "Skip tour"
      assert_no_selector "#tour .tour__card", visible: true

      assert_selector "#help-menu-button", visible: :hidden
      click_button "Open workspace navigation"
      assert_selector "#sidebar.open"

      find("#help-menu-button-sidebar").click
      click_on "Restart tour"

      assert_selector "#tour .tour__card", visible: true
      assert_selector ".tour__progress", text: "Step 1 of 5"
    ensure
      page.current_window.resize_to(1400, 1400)
    end
  end

  private
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

    # Settings and the notification bell ride at the row's trailing edge,
    # where clipping lands first; neither carries an aria-label, so match
    # the edit link and the bell mount instead.
    def assert_trailing_nav_actions
      assert_selector ".room-header__actions a[href$='/edit']", visible: true
      assert_selector ".room-header__actions .button_to_change_notifying", visible: true
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
end
