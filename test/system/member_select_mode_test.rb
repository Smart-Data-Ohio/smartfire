require "application_system_test_case"

class MemberSelectModeTest < ApplicationSystemTestCase
  setup do
    WorkspacePresenceLease.delete_all
    page.current_window.resize_to(1440, 1000)
    sign_in "david@37signals.com"
    join_room rooms(:designers)

    click_button "Show members" if page.has_button?("Show members", wait: 5)
    assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10
  end

  teardown do
    page.current_window.resize_to(1400, 1400)
  end

  test "member rows render without checkboxes and stay inline on desktop and phone" do
    assert_no_selector "#channel-members input[type='checkbox']"
    assert_selector "#channel-members input[type='checkbox']", visible: :all, count: 3
    assert_no_selector "#channel-members [data-member-id='#{users(:david).id}'] input[type='checkbox']", visible: :all
    assert_no_selector "#channel-members [data-multi-select-target='bar']"
    assert_selector "#channel-members [data-multi-select-target='bar'][aria-live='polite']", visible: :all
    assert_member_rows_inline

    page.current_window.resize_to(390, 844)
    assert_no_selector "#channel-members", visible: true
    click_button "Show members"
    assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10
    wait_for_member_panel_animation
    assert_no_selector "#channel-members input[type='checkbox']"
    assert_member_rows_inline
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "a plain click opens the profile card outside selection mode and toggles inside it" do
    row_name_button(users(:jason)).click
    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    assert_selector "#user_card .profile-card__name", text: "Jason"
    find(".profile-card-popover__close").click
    assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10

    ctrl_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }
    assert_selector "#profile-card-popover[hidden]", visible: :all

    row_name_button(users(:kevin)).click
    within_bar { assert_selector "button", text: "Message (2)" }
    assert_selector "#profile-card-popover[hidden]", visible: :all

    row_name_button(users(:jason)).click
    within_bar { assert_selector "button", text: "Message (1)" }
  end

  test "ctrl-click and cmd-click toggle members, and dropping to zero exits the mode" do
    ctrl_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }
    assert_selector "#channel-members input[type='checkbox']", count: 3
    assert_selector "#channel-members input[aria-label='Select Jason']"
    assert_selector "#profile-card-popover[hidden]", visible: :all

    ctrl_click(row_name_button(users(:jason)))
    assert_no_selector "#channel-members [data-multi-select-target='bar']"
    assert_no_selector "#channel-members input[type='checkbox']"

    cmd_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }
    assert_selector "#profile-card-popover[hidden]", visible: :all

    cmd_click(row_name_button(users(:kevin)))
    within_bar { assert_selector "button", text: "Message (2)" }
  end

  test "ctrl-shift-click selects a range across groups and skips your own row" do
    users(:david).user_stars.create!(starred_user: users(:jason))
    refresh_panel
    assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{users(:jason).id}']", visible: :all, wait: 10
    assert_selector "#channel-members [data-member-id='#{users(:david).id}'][data-online='true']", visible: :all, wait: 20
    assert_equal [ users(:jason).id, users(:david).id, users(:jz).id, users(:kevin).id ], visual_member_order

    ctrl_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }

    ctrl_shift_click(row_name_button(users(:jz)))
    within_bar { assert_selector "button", text: "Message (2)" }
    assert_checked_field "select-member-#{users(:jason).id}"
    assert_checked_field "select-member-#{users(:jz).id}"

    # The clicked row is the new anchor: ranging back to Jason from Kevin
    # re-adds Kevin instead of stopping at JZ.
    ctrl_click(row_name_button(users(:kevin)))
    within_bar { assert_selector "button", text: "Message (3)" }
    ctrl_click(row_name_button(users(:kevin)))
    within_bar { assert_selector "button", text: "Message (2)" }
    ctrl_shift_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (3)" }
    assert_checked_field "select-member-#{users(:jason).id}"
    assert_checked_field "select-member-#{users(:jz).id}"
    assert_checked_field "select-member-#{users(:kevin).id}"
  end

  test "esc and the exit button leave selection mode with the panel open" do
    ctrl_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }

    page.send_keys :escape
    assert_no_selector "#channel-members [data-multi-select-target='bar']"
    assert_no_selector "#channel-members input[type='checkbox']"
    assert_selector "#channel-members", visible: true

    ctrl_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }
    within_bar { click_button "Exit selection mode" }
    assert_no_selector "#channel-members [data-multi-select-target='bar']"
    assert_no_selector "#channel-members input[type='checkbox']"
    assert_selector "#channel-members", visible: true
  end

  test "space toggles the focused row without opening the profile card" do
    button = row_name_button(users(:jason))
    page.execute_script("arguments[0].focus()", button)

    button.send_keys(:space)
    within_bar { assert_selector "button", text: "Message (1)" }
    assert_selector "#profile-card-popover[hidden]", visible: :all
    assert_checked_field "select-member-#{users(:jason).id}"

    button.send_keys(:space)
    assert_no_selector "#channel-members [data-multi-select-target='bar']"
    assert_selector "#profile-card-popover[hidden]", visible: :all
  end

  test "selection, mode, and anchor survive a presence re-render" do
    ctrl_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }

    refresh_panel

    within_bar { assert_selector "button", text: "Message (1)" }
    assert_checked_field "select-member-#{users(:jason).id}"

    # The anchor survived too: ranging to Kevin covers Jason through
    # Kevin instead of selecting Kevin alone.
    ctrl_shift_click(row_name_button(users(:kevin)))
    within_bar { assert_selector "button", text: "Message (3)" }
  end

  test "the star row menu works in and out of selection mode" do
    ctrl_click(row_name_button(users(:jason)))
    within_bar { assert_selector "button", text: "Message (1)" }

    member_row(users(:kevin)).right_click
    assert_selector "#member-row-menu", visible: true
    within "#member-row-menu" do
      click_button "☆ Star"
      assert_selector "button", text: "★ Unstar", wait: 10
    end
    assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{users(:kevin).id}']", visible: :all, wait: 10
    within_bar { assert_selector "button", text: "Message (1)" }

    find("#member-row-menu [role='menuitem']").send_keys(:escape)
    assert_no_selector "#member-row-menu"
    page.send_keys :escape
    assert_no_selector "#channel-members [data-multi-select-target='bar']"

    member_row(users(:kevin)).right_click
    assert_selector "#member-row-menu", visible: true
    within "#member-row-menu" do
      click_button "★ Unstar"
      assert_selector "button", text: "☆ Star", wait: 10
    end
    assert_no_selector "#channel-members [aria-label='Starred members']", visible: :all
  end

  test "a phone long-press enters selection mode, then taps toggle and Message submits the set" do
    page.current_window.resize_to(390, 844)
    assert_no_selector "#channel-members", visible: true
    click_button "Show members"
    assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10
    wait_for_member_panel_animation

    row = member_row(users(:jason))
    page.execute_script(<<~JS, row)
      const row = arguments[0]
      const touch = new Touch({ identifier: 1, target: row, clientX: 10, clientY: 10 })
      row.dispatchEvent(new TouchEvent("touchstart", { touches: [ touch ], bubbles: true, cancelable: true }))
    JS
    sleep 0.7
    page.execute_script(<<~JS, row)
      arguments[0].dispatchEvent(new TouchEvent("touchend", { bubbles: true, cancelable: true }))
    JS

    # The release click and the long-press menu must both die: neither the
    # profile card nor the row menu may open.
    page.execute_script(<<~JS, row_name_button(users(:jason)))
      arguments[0].dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 }))
    JS
    page.execute_script(<<~JS, row)
      arguments[0].dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }))
    JS

    assert_checked_field "select-member-#{users(:jason).id}"
    within_bar { assert_selector "button", text: "Message (1)" }
    assert_selector "#profile-card-popover[hidden]", visible: :all
    assert_no_selector "#member-row-menu"

    row_name_button(users(:kevin)).click
    within_bar { assert_selector "button", text: "Message (2)" }
    assert_selector "#profile-card-popover[hidden]", visible: :all

    within_bar { click_button "Message (2)" }

    assert_selector ".room--current", text: "Jason, Kevin", wait: 10
    room = Rooms::Direct.find_for([ users(:david), users(:jason), users(:kevin) ])
    assert_current_path room_path(room)
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  private
    def member_row(user)
      find("#channel-members [data-member-id='#{user.id}']")
    end

    def row_name_button(user)
      find("#channel-members [data-member-id='#{user.id}'] button.profile-card-name")
    end

    def within_bar(&block)
      within("#channel-members [data-multi-select-target='bar']", &block)
    end

    def ctrl_click(element)
      page.driver.browser.action.key_down(:control).click(element.native).key_up(:control).perform
    end

    def ctrl_shift_click(element)
      page.driver.browser.action.key_down(:control).key_down(:shift)
        .click(element.native).key_up(:shift).key_up(:control).perform
    end

    def cmd_click(element)
      page.execute_script(<<~JS, element)
        arguments[0].dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, metaKey: true }))
      JS
    end

    def visual_member_order
      page.evaluate_script(<<~JS)
        Array.from(document.querySelectorAll("#channel-members .member-panel__member"))
          .map((row) => Number(row.dataset.memberId))
      JS
    end

    # Refetch the members and wait until the rows are rebuilt, so the
    # assertions after this prove state survived the re-render.
    def refresh_panel
      page.execute_script(<<~JS)
        window.__oldMemberRow = document.querySelector("#channel-members .member-panel__member")
        window.Stimulus.getControllerForElementAndIdentifier(document.body, "member-panel").refreshPresence()
      JS
      page.document.synchronize(Capybara.default_max_wait_time) do
        raise Capybara::ExpectationNotMet, "member rows were never rebuilt" if page.evaluate_script("window.__oldMemberRow.isConnected")
      end
    end

    # Every row keeps its avatar and identity side by side on one line in
    # both layouts: with no checkbox column by default, and with one while
    # selecting.
    def assert_member_rows_inline
      assert_selector "#channel-members .member-panel__member .member-panel__identity"
      broken = page.evaluate_script(<<~JS)
        Array.from(document.querySelectorAll("#channel-members .member-panel__member")).filter(row => {
          const avatar = row.querySelector(".member-panel__avatar").getBoundingClientRect()
          const identity = row.querySelector(".member-panel__identity").getBoundingClientRect()
          return identity.left < avatar.right || identity.top >= avatar.bottom || identity.width < 80
        }).map(row => row.dataset.memberId)
      JS
      assert_empty broken, "member rows wrapped their names under the avatar"
    end

    # The member panel slides in over a 220 ms transform transition; wait
    # for the slide to settle before tapping row controls.
    def wait_for_member_panel_animation
      page.document.synchronize(Capybara.default_max_wait_time) do
        settled = page.evaluate_script(<<~JS)
          (() => {
            const surface = document.querySelector(".member-panel__surface");
            if (!surface) return true;
            return surface.getAnimations().every((animation) => {
              const timing = animation.effect?.getComputedTiming?.();
              return !(timing && Number.isFinite(timing.endTime) &&
                (animation.playState === "running" || animation.playState === "pending"));
            });
          })()
        JS
        raise Capybara::ExpectationNotMet, "member panel slide-in never settled" unless settled
      end
    end
end
