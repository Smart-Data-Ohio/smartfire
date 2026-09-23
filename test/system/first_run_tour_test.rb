require "application_system_test_case"

class FirstRunTourTest < ApplicationSystemTestCase
  setup do
    users(:jz).update!(tour_completed_at: nil)
  end

  test "a new member is walked through the tour by keyboard and finishing persists" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    assert_selector "#tour .tour__card", visible: true
    assert_selector ".tour__progress", text: "Step 1 of 5"
    assert_selector ".tour__title", text: "Your rooms live here"
    assert_selector "#sidebar.tour__target"

    click_on "Next"
    assert_selector ".tour__progress", text: "Step 2 of 5"
    assert_selector "#composer.tour__target"

    tour_send_keys(:arrow_right)
    assert_selector ".tour__progress", text: "Step 3 of 5"
    assert_selector ".tour__card--center"

    tour_send_keys(:arrow_right)
    assert_selector ".tour__progress", text: "Step 4 of 5"
    assert_selector ".tour__title", text: "Jump anywhere with Ctrl+K"

    tour_send_keys(:arrow_left)
    assert_selector ".tour__progress", text: "Step 3 of 5"

    tour_send_keys(:arrow_right, :arrow_right)
    assert_selector ".tour__progress", text: "Step 5 of 5"
    assert_selector ".tour__title", text: "Shortcuts live under ?"
    assert_selector "#help-menu-button.tour__target"

    click_on "Finish"
    assert_no_selector "#tour .tour__card", visible: true
    assert_not_nil users(:jz).reload.tour_completed_at

    visit room_url(rooms(:designers))
    assert_no_selector "#tour .tour__card", visible: true
  end

  test "escape skips the tour and it never auto-starts again" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    assert_selector "#tour .tour__card", visible: true

    tour_send_keys(:escape)
    assert_no_selector "#tour .tour__card", visible: true
    assert_not_nil users(:jz).reload.tour_completed_at

    visit room_url(rooms(:designers))
    assert_no_selector "#tour .tour__card", visible: true
  end

  test "the tour restarts from the help menu" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    click_on "Skip tour"
    assert_no_selector "#tour .tour__card", visible: true

    find("#help-menu-button").click
    click_on "Restart tour"

    assert_selector "#tour .tour__card", visible: true
    assert_selector ".tour__progress", text: "Step 1 of 5"
  end

  test "members who completed the tour never see it auto-start" do
    users(:jz).touch(:tour_completed_at)

    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    assert_no_selector "#tour .tour__card", visible: true
    assert_selector "#help-menu-button", visible: true
  end

  private
    def tour_send_keys(*keys)
      keys.each do |key|
        find(".tour__card [data-tour-target='next']").send_keys(key)
      end
    end
end
