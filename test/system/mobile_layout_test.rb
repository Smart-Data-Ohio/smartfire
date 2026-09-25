require "application_system_test_case"

# Phone-width regressions: the profile page must not scroll sideways, the
# header on pages outside the workspace shell must stay opaque above
# scrolled content, and every drawer destination must be a shell page
# with one working drawer toggle and itself current in the drawer.
class MobileLayoutTest < ApplicationSystemTestCase
  # Right edges beyond the scroller's content box (clientWidth excludes a
  # classic scrollbar), plus both scroll widths, for the given selectors.
  OVERFLOW_SCRIPT = <<~JS
    (() => {
      const main = document.querySelector("#main-content");
      const limit = main.getBoundingClientRect().left + main.clientWidth + 0.5;
      const offenders = [];
      document.querySelectorAll(arguments[0]).forEach((element) => {
        const rect = element.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) return;
        if (rect.right > limit || rect.left < -0.5) {
          const name = element.id ? `#${element.id}` : element.tagName.toLowerCase() + "." + String(element.className).trim().split(/\\s+/).join(".");
          offenders.push(`${name} [${Math.round(rect.left)}, ${Math.round(rect.right)}] > ${Math.round(limit)}`);
        }
      });
      return {
        documentOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        mainOverflow: main.scrollWidth - main.clientWidth,
        offenders: offenders.slice(0, 10)
      };
    })()
  JS

  PROFILE_ELEMENTS = [
    "#main-content fieldset",
    "#main-content fieldset p",
    "#main-content fieldset .flex > span",
    "#main-content fieldset input:not([type=hidden])",
    "#main-content fieldset select",
    "#main-content fieldset .btn"
  ].join(", ")

  setup do
    sign_in "jz@37signals.com"
  end

  teardown do
    page.current_window.resize_to(1400, 1400)
  end

  test "the profile page fits phone widths without scrolling sideways" do
    [ 320, 375, 414 ].each do |width|
      page.current_window.resize_to(width, 812)
      visit user_profile_path
      assert_selector "#user_quiet_hours_start"

      result = page.evaluate_script(OVERFLOW_SCRIPT, PROFILE_ELEMENTS)
      assert_operator result["documentOverflow"], :<=, 0, "page scrolls sideways at #{width}px: #{result.inspect}"
      assert_operator result["mainOverflow"], :<=, 0, "content scrolls sideways at #{width}px: #{result.inspect}"
      assert_empty result["offenders"], "elements past the right edge at #{width}px"
    end
  end

  test "headers outside the workspace shell stay opaque over scrolled content" do
    page.current_window.resize_to(375, 812)

    [ user_profile_path, edit_account_path, user_path(users(:david)) ].each do |path|
      visit path
      assert_selector "#nav a", text: "Go Back", visible: :all

      page.execute_script("document.querySelector('#main-content').scrollTop = 400")
      header = page.evaluate_script(<<~JS)
        (() => {
          const nav = document.querySelector("#nav");
          const rect = nav.getBoundingClientRect();
          const probe = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
          return { background: getComputedStyle(nav).backgroundColor, hit: Boolean(probe && nav.contains(probe)), height: rect.height };
        })()
      JS

      assert_operator header["height"], :>, 0, "expected a header on #{path}"
      assert_not_includes [ "transparent", "rgba(0, 0, 0, 0)" ], header["background"], "expected an opaque header on #{path}"
      assert_no_match(/rgba\(.*, 0(\.\d+)?\)\z/, header["background"], "expected an opaque header on #{path}")
      assert header["hit"], "expected the header, not the scrolled content, on top at the header on #{path}"
    end
  end

  test "pages outside the workspace shell show no drawer toggle that opens nothing" do
    page.current_window.resize_to(375, 812)

    [ user_profile_path, edit_account_path ].each do |path|
      visit path
      assert_selector "#nav a", text: "Go Back", visible: :all
      assert_no_button "Open workspace navigation"
    end
  end

  test "every drawer destination has one toggle that opens the drawer on itself" do
    page.current_window.resize_to(375, 812)
    visit activity_items_path

    { "People" => users_path, "Agents" => agents_path, "Work threads" => work_threads_path,
      "Saved" => saved_items_path, "Scheduled" => scheduled_messages_path, "Activity inbox" => activity_items_path }.each do |destination, path|
      assert_selector "#main-content"
      click_button "Open workspace navigation"
      assert_selector "#sidebar.open"
      within("#sidebar") { click_link destination }
      assert_no_selector "#sidebar.open"
      assert_current_path path, wait: 10

      visible_controls = page.evaluate_script(<<~JS)
        Array.from(document.querySelectorAll("#nav a, #nav button"))
          .filter((element) => element.getClientRects().length > 0 && !element.closest(".global-search, .help-menu"))
          .map((element) => element.getAttribute("aria-label") || element.textContent.trim())
      JS
      assert_equal [ "Open workspace navigation" ], visible_controls, "expected the drawer toggle alone on #{destination}"

      click_button "Open workspace navigation"
      assert_selector "#sidebar.open"
      assert_selector "#sidebar .workspace-destinations a[aria-current='page']", text: destination, wait: 10
      assert_selector "#sidebar .workspace-destinations a[aria-current='page']", count: 1
      page.send_keys :escape
      assert_no_selector "#sidebar.open"
    end
  end
end
