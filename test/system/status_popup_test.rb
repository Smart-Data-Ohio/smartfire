require "application_system_test_case"

class StatusPopupTest < ApplicationSystemTestCase
  setup do
    page.current_window.resize_to(1440, 1000)
    sign_in "david@37signals.com"
    join_room rooms(:designers)
    assert_selector "#channel-members"
  end

  teardown do
    page.current_window.resize_to(1400, 1400)
  end

  test "setting a status from your own profile card" do
    open_own_profile_card
    within("#user_card") { click_on "Set a status" }

    within "#user_card" do
      assert_selector ".status-popup__title", text: "Set a status"
      select "Do not disturb", from: "status_popup_presence_setting"
      fill_in "status_popup_custom_status_emoji", with: "🚂"
      fill_in "status_popup_custom_status_text", with: "On a train"
      select "1 hour", from: "status_popup_custom_status_expires_in"
      click_button "Save"
    end

    assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10
    assert_selector "#channel-members [data-member-id='#{users(:david).id}']", text: "🚂 On a train", wait: 10
    assert_equal "dnd", users(:david).reload.presence_setting

    visit user_profile_url
    assert_field "user_custom_status_emoji", with: "🚂"
    assert_field "user_custom_status_text", with: "On a train"
    assert_equal "dnd", find("#user_presence_setting").value
  end

  test "your profile in the sidebar opens your card" do
    find("aside#sidebar a.workspace-user").click

    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    within "#user_card" do
      assert_selector ".profile-card__name", text: users(:david).name
      click_on "Set a status"
      assert_selector ".status-popup__title", text: "Set a status"
    end
    assert_no_current_path user_profile_path
  end

  test "clearing a status from the popup" do
    users(:david).update!(custom_status_emoji: "🚂", custom_status_text: "On a train")

    open_own_profile_card
    within("#user_card") { click_on "Set a status" }
    within("#user_card") { click_button "Clear status" }

    assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10
    assert_nil users(:david).reload.custom_status_display
  end

  test "cancel goes back to the card without saving" do
    open_own_profile_card
    within("#user_card") { click_on "Set a status" }

    within "#user_card" do
      fill_in "status_popup_custom_status_text", with: "Never saved"
      click_on "Cancel"
      assert_selector ".profile-card__name", text: users(:david).name
    end
    assert_nil users(:david).reload.custom_status_display
  end

  test "the popup fits a phone screen" do
    page.current_window.resize_to(390, 844)
    click_button "Show members"

    open_own_profile_card
    within("#user_card") { click_on "Set a status" }

    assert_selector "#status_popup_custom_status_text"
    assert page.evaluate_script(<<~JS), "the status popup overflows the card"
      (() => {
        const panel = document.querySelector(".profile-card-popover__panel")
        return panel.scrollWidth <= panel.clientWidth
      })()
    JS
  end

  private
    def open_own_profile_card
      within "#channel-members [data-member-id='#{users(:david).id}']" do
        find("button.profile-card-name").click
      end
      assert_selector "#profile-card-popover:not([hidden])", wait: 10
      assert_selector "#user_card .profile-card__name", text: users(:david).name
    end
end
