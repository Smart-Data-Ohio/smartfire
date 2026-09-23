require "application_system_test_case"

class StarredPeopleTest < ApplicationSystemTestCase
  setup do
    WorkspacePresenceLease.delete_all
    @forgery_protection = ActionController::Base.allow_forgery_protection
    ActionController::Base.allow_forgery_protection = true
    page.current_window.resize_to(1440, 1000)
    sign_in "jz@37signals.com"
    join_room rooms(:designers)
    assert_selector "#channel-members"
  end

  teardown do
    ActionController::Base.allow_forgery_protection = @forgery_protection
    page.current_window.resize_to(1400, 1400)
  end

  test "starring from the profile card floats the person into a Starred group" do
    kevin = users(:kevin)
    assert_no_selector "#channel-members [aria-label='Starred members']", visible: :all

    open_profile_card_for kevin
    within "#user_card" do
      click_button "☆ Star"
      assert_selector "button", text: "★ Unstar", wait: 10
    end

    assert_starred kevin
    assert_member_rows_inline
    close_profile_card

    open_profile_card_for kevin
    within "#user_card" do
      click_button "★ Unstar"
      assert_selector "button", text: "☆ Star", wait: 10
    end
    close_profile_card

    assert_unstarred kevin
    assert_member_rows_inline
  end

  test "the member row menu stars and unstars, by mouse and keyboard" do
    kevin = users(:kevin)

    find("#channel-members [data-member-id='#{kevin.id}']").right_click
    assert_selector "#member-row-menu", visible: true
    assert_focused "#member-row-menu [role='menuitem']"

    within "#member-row-menu" do
      click_button "☆ Star"
      assert_selector "button", text: "★ Unstar", wait: 10
    end
    assert_starred kevin

    within "#member-row-menu" do
      click_button "★ Unstar"
      assert_selector "button", text: "☆ Star", wait: 10
    end
    assert_unstarred kevin

    find("#member-row-menu [role='menuitem']").send_keys(:escape)
    assert_no_selector "#member-row-menu", visible: true
    assert_row_trigger_focused kevin

    # Keyboard path: Shift+F10 on a focused row control opens the menu.
    page.execute_script(<<~JS, kevin.id)
      document.querySelector(`#channel-members [data-member-id='${arguments[0]}'] .profile-card-name`).focus()
    JS
    press_keys(:shift, :f10)
    assert_selector "#member-row-menu", visible: true
    assert_focused "#member-row-menu [role='menuitem']"
    within "#member-row-menu" do
      click_button "☆ Star"
    end
    assert_starred kevin
    find("#member-row-menu [role='menuitem']").send_keys(:escape)
    assert_no_selector "#member-row-menu", visible: true

    # Your own row offers no menu: there is nothing to star.
    find("#channel-members [data-member-id='#{users(:jz).id}']").right_click
    assert_no_selector "#member-row-menu", visible: true

    assert_member_rows_inline
  end

  test "the Starred group works on a phone" do
    kevin = users(:kevin)
    page.current_window.resize_to(390, 844)
    assert_no_selector "#channel-members", visible: true

    click_button "Show members"
    assert_selector "#channel-members [data-member-id='#{kevin.id}']", wait: 10

    open_profile_card_for kevin
    within "#user_card" do
      click_button "☆ Star"
      assert_selector "button", text: "★ Unstar", wait: 10
    end
    close_profile_card

    assert_starred kevin
    assert_member_rows_inline
    assert_no_horizontal_overflow
  end

  private
    def open_profile_card_for(user)
      within "#channel-members [data-member-id='#{user.id}']" do
        find("button.profile-card-name").click
      end
      assert_selector "#profile-card-popover:not([hidden])", wait: 10
      assert_selector "#user_card .profile-card__name", text: user.name
    end

    def close_profile_card
      find(".profile-card-popover__close").click
      assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10
    end

    def assert_starred(user)
      assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'][data-starred='true']",
        text: user.name, visible: :all, wait: 10
      assert_no_selector "#channel-members [aria-label='Online members'] [data-member-id='#{user.id}']", visible: :all
      assert_no_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}']", visible: :all
      assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'] .member-panel__presence",
        visible: :all
    end

    def assert_unstarred(user)
      assert_no_selector "#channel-members [aria-label='Starred members']", visible: :all
      assert_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}'][data-starred='false']",
        text: user.name, visible: :all, wait: 10
    end

    def assert_row_trigger_focused(user)
      assert page.evaluate_script(<<~JS, user.id), "expected focus to return to the member row"
        document.querySelector(`#channel-members [data-member-id='${arguments[0]}']`).contains(document.activeElement)
      JS
    end

    # Every row keeps its avatar and identity side by side on one line:
    # the selection checkbox must not push the name onto a narrow second
    # grid row.
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

    def assert_no_horizontal_overflow
      assert page.evaluate_script("document.documentElement.scrollWidth <= innerWidth + 1"), "the workspace overflows the viewport horizontally"
    end
end
