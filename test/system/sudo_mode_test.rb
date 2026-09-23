require "application_system_test_case"

class SudoModeTest < ApplicationSystemTestCase
  setup do
    sign_in "david@37signals.com"
  end

  test "one prompt, then the action continues automatically" do
    visit edit_account_path
    before = find_field("invite_url").value
    assert_not_empty before

    click_button "Regenerate join link"

    # The sensitive action redirects to the sudo prompt.
    assert_selector "h1", text: "Confirm it's you", wait: 10
    fill_in "password", with: "secret123456"
    click_button "Confirm"

    # Confirmation replays the stashed request without another click.
    assert_selector "#invite_url", wait: 10
    after = find_field("invite_url").value
    assert_not_empty after
    assert_not_equal before, after
  end

  test "a wrong password keeps the action gated" do
    visit edit_account_path
    before = find_field("invite_url").value

    click_button "Regenerate join link"
    assert_selector "h1", text: "Confirm it's you", wait: 10
    fill_in "password", with: "wrong"
    click_button "Confirm"

    assert_selector ".flash", text: "Confirmation failed", wait: 10

    visit edit_account_path
    assert_equal before, find_field("invite_url").value
  end
end
