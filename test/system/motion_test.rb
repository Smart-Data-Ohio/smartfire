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
