require "test_helper"

class KeyboardShortcutsSheetTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "sheet lists the global navigation shortcuts" do
    get room_url(rooms(:designers))
    assert_response :success

    assert_select "#keyboard-shortcuts", text: /quick switcher/i
    assert_select "#keyboard-shortcuts", text: /previous \/ next room/i
    assert_select "#keyboard-shortcuts", text: /mark current room read/i
  end

  test "sheet lists the composer's up-arrow edit shortcut" do
    get room_url(rooms(:designers))
    assert_response :success

    assert_select "#keyboard-shortcuts", text: /edit your last message/i
  end
end
