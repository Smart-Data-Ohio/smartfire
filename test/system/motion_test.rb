require "application_system_test_case"

# The layout renders data-test-motion="off" in the test environment, so every
# token-driven transition is instant and assertions never catch mid-flight
# motion. These tests pin that switch, then turn motion back on to prove the
# mobile drawer still lands in place with focus where it belongs.
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

  test "mobile drawer slides in, lands in place, and returns focus with motion on" do
    page.current_window.resize_to(390, 844)
    page.evaluate_script("document.documentElement.removeAttribute('data-test-motion')")

    # The exit animation keeps the drawer rendered while it slides out.
    assert_includes sidebar_transition_properties, "visibility"

    click_button "Open workspace navigation"
    assert_selector "#sidebar.open"
    wait_for_drawer_to_land

    assert_equal "1", page.evaluate_script("getComputedStyle(document.querySelector('#sidebar')).opacity")
    assert page.evaluate_script("document.querySelector('#sidebar').contains(document.activeElement)"),
      "expected focus to land inside the open drawer"

    page.send_keys :escape
    assert_no_selector "#sidebar.open"
    wait_until("expected focus to return to the opener") do
      page.evaluate_script("document.activeElement?.getAttribute('aria-label')") == "Open workspace navigation"
    end
  end

  private

  def motion_token(name)
    page.evaluate_script("getComputedStyle(document.documentElement).getPropertyValue('#{name}').trim()")
  end

  def sidebar_transition_properties
    page.evaluate_script("getComputedStyle(document.querySelector('#sidebar')).transitionProperty")
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
