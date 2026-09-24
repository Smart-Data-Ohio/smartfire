require "application_system_test_case"

class SidebarRoomMenuTest < ApplicationSystemTestCase
  test "an admin deletes a channel from the room menu" do
    sign_in "david@37signals.com"
    join_room rooms(:hq)
    room = rooms(:designers)

    open_room_menu room
    within("#room-menu") { click_on "Delete…" }

    within(".room-menu__confirm") do
      assert_text "Delete #Designers and all its messages? This can't be undone."
      click_on "Delete"
    end

    assert_no_selector "#sidebar a[data-room-id='#{room.id}']", wait: 10
    assert_selector ".flash", text: "Deleted #Designers", wait: 10
    assert_predicate room.reload, :deleted?
    assert AuditLog.where(action: "room.destroy", target_id: room.id).exists?
  end

  test "the creator sees Delete on their own room but not on others" do
    own = Rooms::Closed.create_for({ name: "JZ Club", creator: users(:jz) }, users: [ users(:jz), users(:david) ])
    sign_in "jz@37signals.com"
    join_room rooms(:hq)

    open_room_menu own
    within("#room-menu") { assert_selector "button", text: "Delete…" }
    page.send_keys :escape
    assert_no_selector "#room-menu:not([hidden])", wait: 5

    open_room_menu rooms(:designers)
    within("#room-menu") { assert_no_selector "button", text: "Delete…" }
  end

  test "a plain member does not see Delete" do
    sign_in "jz@37signals.com"
    join_room rooms(:hq)

    open_room_menu rooms(:designers)
    within("#room-menu") { assert_no_selector "button", text: "Delete…" }
  end

  test "Cancel and Escape keep the room" do
    sign_in "david@37signals.com"
    join_room rooms(:hq)
    room = rooms(:designers)

    open_room_menu room
    within("#room-menu") { click_on "Delete…" }
    within(".room-menu__confirm") { click_on "Cancel" }

    assert_no_selector ".room-menu__confirm[open]", wait: 5
    assert_selector "#sidebar a[data-room-id='#{room.id}']"
    assert_not_predicate room.reload, :deleted?

    open_room_menu room
    within("#room-menu") { click_on "Delete…" }
    assert_selector ".room-menu__confirm[open]", wait: 5
    page.send_keys :escape

    assert_no_selector ".room-menu__confirm[open]", wait: 5
    assert_not_predicate room.reload, :deleted?
  end

  test "deleting the room you are in navigates you home" do
    sign_in "david@37signals.com"
    room = rooms(:designers)
    join_room room

    open_room_menu room
    within("#room-menu") { click_on "Delete…" }
    within(".room-menu__confirm") { click_on "Delete" }

    page.document.synchronize(10) do
      raise Capybara::ExpectationNotMet if current_path == room_path(room)
    end
    assert_selector ".flash", text: "Deleted #Designers", wait: 10
    assert_predicate room.reload, :deleted?
  end

  test "Delete is offered on every room kind for admins" do
    david = users(:david)
    board = Rooms::Board.create_for({ name: "Launch", creator: david }, users: [ david ])
    voice = Rooms::Voice.create_for({ name: "Lounge", creator: david }, users: [ david ])
    stage = Rooms::Stage.create_for({ name: "Town Hall", creator: david }, users: [ david ])

    sign_in "david@37signals.com"
    join_room rooms(:hq)

    [ rooms(:hq), rooms(:designers), board, voice, stage ].each do |room|
      open_room_menu room
      within("#room-menu") { assert_selector "button", text: "Delete…" }
      page.send_keys :escape
      assert_no_selector "#room-menu:not([hidden])", wait: 5
    end

    # The full flow works past channels too: delete the stage from its row.
    open_room_menu stage
    within("#room-menu") { click_on "Delete…" }
    within(".room-menu__confirm") { click_on "Delete" }

    assert_no_selector "#sidebar a[data-room-id='#{stage.id}']", wait: 10
    assert_predicate stage.reload, :deleted?
  end

  test "a group dm shows Delete only for admins" do
    group = Rooms::Direct.create_for(
      { name: "Weekend Plans", creator: users(:jz) },
      users: [ users(:jz), users(:kevin), users(:david) ]
    )

    sign_in "david@37signals.com"
    join_room rooms(:hq)
    open_room_menu group
    within("#room-menu") { assert_selector "button", text: "Delete…" }

    sign_in "jz@37signals.com"
    join_room rooms(:hq)
    open_room_menu group
    within("#room-menu") { assert_no_selector "button", text: "Delete…" }
  end

  test "one-to-one dms follow the server delete rule" do
    sign_in "kevin@37signals.com"
    join_room rooms(:hq)

    # Kevin created bender_and_kevin, so they may delete it; david_and_kevin
    # belongs to David.
    open_room_menu rooms(:bender_and_kevin)
    within("#room-menu") { assert_selector "button", text: "Delete…" }
    page.send_keys :escape
    assert_no_selector "#room-menu:not([hidden])", wait: 5

    open_room_menu rooms(:david_and_kevin)
    within("#room-menu") { assert_no_selector "button", text: "Delete…" }
  end

  test "Delete is reachable by arrow keys" do
    sign_in "david@37signals.com"
    join_room rooms(:hq)

    row_for(rooms(:designers)).send_keys(:shift, :f10)
    assert_selector "#room-menu:not([hidden])", wait: 5

    page.send_keys(:end)
    assert_focused "#room-menu [data-room-menu-target='deleteAction']"

    page.send_keys(:enter)
    assert_selector ".room-menu__confirm[open]", wait: 5
  end

  test "deleting from the long-press menu on phones" do
    sign_in "david@37signals.com"
    join_room rooms(:hq)
    room = rooms(:designers)
    page.current_window.resize_to(390, 844)
    click_button "Open workspace navigation"
    assert_selector "#sidebar.open", wait: 5

    # A phone long-press arrives as a contextmenu event; dispatching it
    # on the drawer row drives the same handler without a touch device.
    page.execute_script(<<~'JS', room.id)
      const row = document.querySelector(`#sidebar a[data-room-id="${arguments[0]}"]`);
      row.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    JS
    assert_selector "#room-menu:not([hidden])", wait: 5
    within("#room-menu") { click_on "Delete…" }

    within(".room-menu__confirm") { click_on "Delete" }

    assert_no_selector "#sidebar a[data-room-id='#{room.id}']", wait: 10
    assert_predicate room.reload, :deleted?
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "a member leaves an open channel and rejoins through the join page" do
    sign_in "jz@37signals.com"
    room = rooms(:hq)
    join_room rooms(:designers)

    open_room_menu room
    within("#room-menu") { assert_selector "button", text: "Leave" }
    within("#room-menu") { click_on "Leave" }

    within(".room-menu__confirm") do
      assert_text "Leave #HQ? You'll stop seeing it in your sidebar."
      click_on "Leave"
    end

    assert_no_selector "#sidebar a[data-room-id='#{room.id}']", wait: 10
    assert_not room.reload.user_ids.include?(users(:jz).id)
    assert_not_predicate room, :deleted?

    visit room_path(room)
    assert_selector "h2", text: "#HQ", wait: 10
    click_on "Join channel"

    assert_selector "#sidebar a[data-room-id='#{room.id}']", wait: 10
    assert room.reload.user_ids.include?(users(:jz).id)
  end

  test "leaving a private channel warns about being re-added" do
    sign_in "jz@37signals.com"
    room = rooms(:designers)
    join_room rooms(:hq)

    open_room_menu room
    within("#room-menu") { click_on "Leave" }

    within(".room-menu__confirm") do
      assert_text "Leave #Designers? You'll stop seeing it in your sidebar. Someone will need to add you back."
      click_on "Leave"
    end

    assert_no_selector "#sidebar a[data-room-id='#{room.id}']", wait: 10
    assert_not_predicate room.reload, :deleted?

    entry = AuditLog.where(action: "room.membership.change", target_id: room.id).last
    assert_equal users(:jz).id, entry.actor_id
    assert_equal [ "JZ" ], entry.details["revoked"]
  end

  test "leaving the room you are in sends you home" do
    sign_in "jz@37signals.com"
    room = rooms(:hq)
    join_room room

    open_room_menu room
    within("#room-menu") { click_on "Leave" }
    within(".room-menu__confirm") { click_on "Leave" }

    page.document.synchronize(10) do
      raise Capybara::ExpectationNotMet if current_path == room_path(room)
    end
    assert_not room.reload.user_ids.include?(users(:jz).id)
  end

  test "the last member out leaves the room behind for no one to lose" do
    room = Rooms::Closed.create_for({ name: "Solo", creator: users(:jz) }, users: [ users(:jz) ])
    sign_in "jz@37signals.com"
    join_room rooms(:hq)

    open_room_menu room
    within("#room-menu") { click_on "Leave" }
    within(".room-menu__confirm") { click_on "Leave" }

    assert_no_selector "#sidebar a[data-room-id='#{room.id}']", wait: 10
    assert_not_predicate room.reload, :deleted?
    assert_empty room.memberships
  end

  test "Leave is offered on every room kind and never deletes" do
    jz = users(:jz)
    david = users(:david)
    board = Rooms::Board.create_for({ name: "Launch", creator: david }, users: [ david, jz ])
    voice = Rooms::Voice.create_for({ name: "Lounge", creator: david }, users: [ david, jz ])
    stage = Rooms::Stage.create_for({ name: "Town Hall", creator: david }, users: [ david, jz ])

    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    [ rooms(:hq), rooms(:designers), board, voice, stage ].each do |room|
      open_room_menu room
      within("#room-menu") { click_on "Leave" }
      within(".room-menu__confirm") { click_on "Leave" }

      assert_no_selector "#sidebar a[data-room-id='#{room.id}']", wait: 10
      room.reload
      assert_not_predicate room, :deleted?
      assert_not room.user_ids.include?(jz.id)
    end
  end

  test "a member leaves a group dm from the menu" do
    group = Rooms::Direct.create_for(
      { name: "Weekend Plans", creator: users(:david) },
      users: [ users(:david), users(:jason), users(:jz) ]
    )
    sign_in "jz@37signals.com"
    join_room rooms(:hq)

    open_room_menu group
    within("#room-menu") { click_on "Leave" }
    within(".room-menu__confirm") { click_on "Leave" }

    assert_no_selector "#sidebar a[data-room-id='#{group.id}']", wait: 10
    assert_not group.reload.user_ids.include?(users(:jz).id)
    assert_not_predicate group, :deleted?
  end

  private
    def row_for(room)
      find("#sidebar a[data-room-id='#{room.id}']")
    end

    def open_room_menu(room)
      row_for(room).right_click
      assert_selector "#room-menu:not([hidden])", wait: 5
    end
end
