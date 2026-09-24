require "application_system_test_case"

# The layout renders data-test-motion="off" in the test environment, so every
# token-driven transition is instant and assertions never catch mid-flight
# motion. These tests pin that switch, then turn motion back on to prove the
# mobile drawer animates in (not just lands), and prove the room list keeps
# its scroll position across close and reopen.
class MotionTest < ApplicationSystemTestCase
  setup do
    page.current_window.resize_to(1400, 1400)
    sign_in "jz@37signals.com"
    join_room rooms(:hq)
  end

  teardown do
    page.current_window.resize_to(1400, 1400)
  end

  test "motion is off by default in the test environment" do
    assert_equal "off", page.evaluate_script("document.documentElement.dataset.testMotion")
    assert_equal "0ms", motion_token("--motion-medium")
    assert_equal "0ms", motion_token("--motion-fast")
    assert_equal "0ms", motion_token("--motion-quick")
  end

  test "mobile drawer animates in, lands in place, and returns focus with motion on" do
    page.current_window.resize_to(390, 844)
    page.evaluate_script("document.documentElement.removeAttribute('data-test-motion')")
    # Stretch the slide so the mid-transition sample cannot miss it.
    page.evaluate_script("document.documentElement.style.setProperty('--motion-medium', '30s')")

    track_transition_runs
    # Drop anything the desktop-to-mobile resize started: only the open may
    # record from here.
    page.evaluate_script("(() => { window.__motionRuns = []; })()")
    click_button "Open workspace navigation"
    assert_selector "#sidebar.open"

    # The surface must travel from off-canvas to landed, not render landed:
    # a drawer without a before-change style (display:none while closed)
    # paints the open state immediately and skips the slide.
    start_tx = drawer_surface_tx
    assert start_tx < -1, "expected the drawer to start off-canvas, got tx=#{start_tx}"
    wait_until("expected the drawer surface to start sliding") do
      tx = drawer_surface_tx
      tx && tx > start_tx + 0.5
    end
    mid_tx = drawer_surface_tx
    assert mid_tx < -1, "expected a mid-travel sample, the drawer already landed (tx=#{mid_tx})"

    assert_running_transition "#sidebar .sidebar__container", "transform"
    wait_until("expected enter transitions to run") { enter_transitions_ran? }

    # The stretched duration is captured at transition start: dropping the
    # override only affects later transitions, so finish this one via the
    # Web Animations API instead of waiting out 30 seconds.
    page.evaluate_script("(() => { document.querySelector('#sidebar').getAnimations().forEach((animation) => animation.finish()); document.querySelector('#sidebar .sidebar__container').getAnimations().forEach((animation) => animation.finish()); })()")
    page.evaluate_script("document.documentElement.style.removeProperty('--motion-medium')")
    wait_for_drawer_to_land

    assert_equal "1", page.evaluate_script("getComputedStyle(document.querySelector('#sidebar')).opacity")
    wait_until("expected focus to land inside the open drawer") do
      page.evaluate_script("document.querySelector('#sidebar').contains(document.activeElement)")
    end

    page.send_keys :escape
    assert_no_selector "#sidebar.open"
    wait_until("expected focus to return to the opener") do
      page.evaluate_script("document.activeElement?.getAttribute('aria-label')") == "Open workspace navigation"
    end
  end

  test "member selection mode moves no rows and resizes nothing" do
    click_button "Show members" if page.has_button?("Show members", wait: 5)
    assert_selector "#channel-members .member-panel__member", minimum: 2, wait: 10

    # The checkbox column is reserved up front: entering selection mode
    # only fades and scales the boxes in, so avatar columns never shift.
    assert_no_selector "#channel-members input[type='checkbox']"
    lefts_before = member_avatar_lefts
    content_height_before = member_content_height

    ctrl_click(find("#channel-members [data-member-id='#{users(:jason).id}'] button.profile-card-name"))
    assert_selector "#channel-members input[type='checkbox']"
    assert_selector "#channel-members [data-multi-select-target='bar']"
    assert_equal lefts_before, member_avatar_lefts, "expected selection mode to shift no rows"
    assert_equal content_height_before, member_content_height, "expected the overlaid bar to resize nothing"

    # The bottom rows stay clickable under the overlaid bar (clearance
    # pads the content clear of it).
    boxes = all("#channel-members input[type='checkbox']").reject { |box| box[:id] == "select-member-#{users(:jason).id}" }
    boxes.last.click
    assert_selector "#channel-members [data-multi-select-target='messageButton']", text: "Message (2)"
    assert_equal lefts_before, member_avatar_lefts, "expected a second selection to shift no rows"
    assert_equal content_height_before, member_content_height, "expected a second selection to resize nothing"

    all("#channel-members input[type='checkbox']").each { |box| box.click if box.checked? }
    assert_no_selector "#channel-members [data-multi-select-target='bar']"
    assert_equal lefts_before, member_avatar_lefts, "expected leaving selection mode to shift no rows"
    assert_equal content_height_before, member_content_height, "expected hiding the bar to resize nothing"
  end

  test "people directory bar shifts no rows when toggling" do
    visit users_path
    assert_selector ".people-directory__row", minimum: 2

    # The bar is the last child, so toggling it only grows or shrinks
    # trailing space; no row may move.
    tops_before = directory_row_tops
    first(".people-directory__row input[type='checkbox']").click
    assert_selector ".multi-select-bar", visible: true
    assert_equal tops_before, directory_row_tops, "expected showing the bar to move no rows"

    all(".people-directory__row input[type='checkbox']").each { |box| box.click if box.checked? }
    assert_no_selector ".multi-select-bar"
    assert_equal tops_before, directory_row_tops, "expected hiding the bar to move no rows"
  end

  test "people directory bar stays stuck while scrolling" do
    12.times { |index| User.create!(name: "Sticky User #{index}", email_address: "sticky#{index}@example.test") }
    page.current_window.resize_to(1400, 400)
    visit users_path
    assert_selector ".people-directory__row", minimum: 10
    assert page.evaluate_script("(() => { const main = document.querySelector('#main-content'); return main.scrollHeight > main.clientHeight + 100; })()"),
      "expected the list to scroll so the sticky check is real"

    first(".people-directory__row input[type='checkbox']").click
    assert_selector ".multi-select-bar", visible: true

    # Mid-list: the bar must stick to the viewport bottom instead of
    # scrolling away with the list end. (At the very top the bar's
    # containing block clamps it a few pixels lower; scrolled in, the
    # scrollport edge alone decides.)
    page.execute_script("document.querySelector('#main-content').scrollTop = 100")
    wait_until("expected the list to scroll") do
      page.evaluate_script("document.querySelector('#main-content').scrollTop") == 100
    end
    geometry = page.evaluate_script(<<~JS)
      (() => {
        const bar = document.querySelector(".multi-select-bar").getBoundingClientRect();
        const main = document.querySelector("#main-content").getBoundingClientRect();
        return { barBottom: bar.bottom, mainBottom: main.bottom };
      })()
    JS
    assert_operator geometry["barBottom"], :<=, geometry["mainBottom"] + 1,
      "expected the bar stuck in view, got #{geometry.inspect}"
  end

  test "mobile drawer keeps the room list scroll position across close and reopen" do
    user = users(:jz)
    15.times { |index| Rooms::Closed.create!(name: "Scroll room #{index}", creator: user).memberships.create!(user: user) }
    join_room rooms(:hq)

    page.current_window.resize_to(390, 844)
    click_button "Open workspace navigation"
    assert_selector "#sidebar.open"

    # Let the enter focus land (and its scroll-into-view finish) before
    # measuring scroll: a late focus scroll would otherwise yank the list
    # mid-sample. Blur right after so nothing else scrolls it either.
    wait_until("expected focus to move into the drawer") do
      page.evaluate_script("document.querySelector('#sidebar').contains(document.activeElement)")
    end
    page.evaluate_script("(() => { document.activeElement.blur(); })()")

    scroller = "#sidebar .sidebar__scroll"
    assert page.evaluate_script("document.querySelector('#{scroller}').scrollHeight > document.querySelector('#{scroller}').clientHeight"),
      "expected the room list to overflow so the scroll check is real"

    # Reopening focuses the current room, which scrolls it into view: probe
    # far from that focus-determined landing (160px here) so a restore that
    # merely focuses cannot masquerade as preservation.
    max_scroll = page.evaluate_script("document.querySelector('#{scroller}').scrollHeight - document.querySelector('#{scroller}').clientHeight")
    assert max_scroll >= 400, "expected room for a 400px scroll, got #{max_scroll}px"
    # The list smooth-scrolls programmatic sets, so wait until the scroll
    # fully lands before snapshotting; a mid-flight sample would keep
    # travelling across the close and reopen below.
    page.execute_script("document.querySelector('#{scroller}').scrollTop = 400")
    wait_until("expected the room list to scroll") do
      page.evaluate_script("document.querySelector('#{scroller}').scrollTop") == 400
    end
    before = page.evaluate_script("document.querySelector('#{scroller}').scrollTop")
    assert_equal 400, before

    page.send_keys :escape
    assert_no_selector "#sidebar.open"

    # The boxes must survive the close: Chrome's same-element scroll-offset
    # retention restores the position on reopen even over display:none, so
    # the reopen check below passes either way there. Reading while closed
    # pins the structural guarantee instead (visibility keeps boxes,
    # display:none destroys them and reads back 0).
    closed = page.evaluate_script("document.querySelector('#{scroller}').scrollTop")
    assert_equal before, closed, "expected the room list to keep its scroll position while closed"

    click_button "Open workspace navigation"
    assert_selector "#sidebar.open"
    after = page.evaluate_script("document.querySelector('#{scroller}').scrollTop")
    assert_equal before, after, "expected the room list to keep its scroll position"
  end

  private

  def motion_token(name)
    page.evaluate_script("getComputedStyle(document.documentElement).getPropertyValue('#{name}').trim()")
  end

  def ctrl_click(element)
    page.driver.browser.action.key_down(:control).click(element.native).key_up(:control).perform
  end

  def member_avatar_lefts
    page.evaluate_script(<<~JS)
      Array.from(document.querySelectorAll("#channel-members .member-panel__member .member-panel__avatar"))
        .map((avatar) => avatar.getBoundingClientRect().left)
    JS
  end

  def member_content_height
    page.evaluate_script("document.querySelector('#channel-members .member-panel__content').clientHeight")
  end

  def directory_row_tops
    page.evaluate_script(<<~JS)
      Array.from(document.querySelectorAll(".people-directory__row")).map((row) => row.getBoundingClientRect().top)
    JS
  end

  def drawer_surface_tx
    page.evaluate_script(<<~JS)
      (() => {
        const transform = getComputedStyle(document.querySelector("#sidebar .sidebar__container")).transform;
        if (transform === "none") return 0;
        const match = transform.match(/matrix\((.+)\)/);
        return match ? Number(match[1].split(",").map((part) => part.trim())[4]) : null;
      })()
    JS
  end

  def track_transition_runs
    result = page.evaluate_script(<<~JS)
      (() => {
        window.__motionRuns = [];
        const sidebar = document.querySelector("#sidebar");
        const surface = document.querySelector("#sidebar .sidebar__container");
        let attached = 0;
        if (sidebar) {
          sidebar.addEventListener("transitionrun", (event) => { window.__motionRuns.push("drawer:" + event.propertyName); });
          attached += 1;
        }
        if (surface) {
          surface.addEventListener("transitionrun", (event) => { window.__motionRuns.push("surface:" + event.propertyName); });
          attached += 1;
        }
        return { attached: attached, hasSidebar: !!sidebar, hasSurface: !!surface };
      })()
    JS
    raise "transition listeners did not attach: #{result.inspect}" unless result["attached"] == 2
  end

  def enter_transitions_ran?
    runs = page.evaluate_script("window.__motionRuns || []")
    runs.include?("drawer:opacity") && runs.include?("surface:transform")
  end

  def assert_running_transition(selector, property)
    running = page.evaluate_script(<<~JS, selector)
      document.querySelector(arguments[0]).getAnimations()
        .filter((animation) => animation.playState === "running")
        .map((animation) => animation.transitionProperty)
    JS
    assert_includes running, property, "expected a running #{property} transition on #{selector}, got #{running.inspect}"
  end

  def wait_for_drawer_to_land
    wait_until("expected the drawer surface to land in place") do
      transform = page.evaluate_script("getComputedStyle(document.querySelector('#sidebar .sidebar__container')).transform")
      transform == "none" || transform == "matrix(1, 0, 0, 1, 0, 0)"
    end
  end

  def wait_until(message, timeout: 5)
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
    loop do
      return if yield
      flunk message if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
      sleep 0.05
    end
  end
end
