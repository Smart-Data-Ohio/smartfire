require "application_system_test_case"

class SidebarOrganizeTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
  end

  test "favouriting from the room menu moves the room to Favourites" do
    join_room rooms(:hq)

    open_room_menu rooms(:designers)
    within "#room-menu" do
      click_on "Add to favourites"
    end

    within "#favorite_rooms" do
      assert_text "Designers", wait: 10
    end
    within "#shared_rooms" do
      assert_no_text "Designers", wait: 5
    end
  end

  test "favourites reorder from the keyboard with move up and down" do
    join_room rooms(:hq)

    open_room_menu rooms(:designers)
    within("#room-menu") { click_on "Add to favourites" }
    within("#favorite_rooms") { assert_text "Designers", wait: 10 }

    open_room_menu rooms(:hq)
    within("#room-menu") { click_on "Add to favourites" }
    within("#favorite_rooms") { assert_text "HQ", wait: 10 }

    assert_equal [ "Designers", "HQ" ], favorite_names

    row_for(rooms(:hq)).send_keys(:shift, :f10)
    assert_selector "#room-menu:not([hidden])", wait: 5
    within("#room-menu") { click_on "Move up" }

    assert_selector "#favorite_rooms > a:first-child[data-room-id='#{rooms(:hq).id}']", wait: 10
    assert_equal [ "HQ", "Designers" ], favorite_names
  end

  test "favourites reorder with drag and drop" do
    join_room rooms(:hq)

    users(:jz).memberships.find_by!(room: rooms(:designers)).favorite!
    users(:jz).memberships.find_by!(room: rooms(:hq)).favorite!
    visit room_path(rooms(:hq))
    assert_selector "#favorite_rooms", wait: 10
    assert_equal [ "Designers", "HQ" ], favorite_names

    drag_favorite_to_top rooms(:hq)

    assert_selector "#favorite_rooms > a:first-child[data-room-id='#{rooms(:hq).id}']", wait: 10
    assert_equal [ "HQ", "Designers" ], favorite_names
  end

  test "muted rooms dim and stay quiet until mentioned" do
    sign_in "david@37signals.com"
    designers = rooms(:designers)
    join_room rooms(:pets)

    open_room_menu designers
    within("#room-menu") { click_on "Mute" }
    assert_selector "#sidebar a.muted[data-room-id='#{designers.id}']", wait: 10

    # The watercooler badge proves delivery is flowing while the muted
    # room stays quiet for the same plain message.
    designers.root_messages.create!(creator: users(:kevin), body: "Muted noise", client_message_id: "mute-quiet-1")
    rooms(:watercooler).root_messages.create!(creator: users(:kevin), body: "Loud hello", client_message_id: "mute-quiet-2")
    assert_room_unread rooms(:watercooler)
    assert_room_read designers

    designers.root_messages.create!(
      creator: users(:kevin), body: "Hey #{mention_attachment_for(:david)}", client_message_id: "mute-quiet-3"
    )
    assert_room_unread designers
  end

  test "channel categories organize, collapse and persist" do
    join_room rooms(:hq)

    find(".room-category__new-toggle").click
    within ".room-category__new-form" do
      fill_in "New category name", with: "Team"
      click_on "Create"
    end
    assert_selector ".sidebar-section--category h2", text: /team/i, wait: 10

    open_room_menu rooms(:designers)
    within("#room-menu") { click_on "Team" }
    assert_selector "#sidebar [data-category-drop]", wait: 10

    within_category "Team" do
      assert_text "Designers", wait: 10
    end
    within "#shared_rooms" do
      assert_no_text "Designers", wait: 5
    end

    within_category "Team" do
      click_on "Collapse Team"
    end
    assert_no_selector "#sidebar .sidebar-list:not([hidden]) a[data-room-id='#{rooms(:designers).id}']", wait: 5

    visit room_path(rooms(:hq))
    assert_no_selector "#sidebar .sidebar-list:not([hidden]) a[data-room-id='#{rooms(:designers).id}']", wait: 10
  end

  test "categories rename and delete" do
    category = users(:jz).room_categories.create!(name: "Team", position: 1)
    join_room rooms(:hq)

    within_category "Team" do
      find(".room-category__rename summary").click
      fill_in "New name for Team", with: "Squad"
      click_on "Save"
    end
    assert_selector ".sidebar-section--category h2", text: /squad/i, wait: 10

    users(:jz).memberships.find_by!(room: rooms(:designers)).update!(room_category: category)
    visit room_path(rooms(:hq))

    within_category "Squad" do
      assert_text "Designers", wait: 10
      accept_confirm do
        click_on "Delete Squad"
      end
    end

    within "#shared_rooms" do
      assert_text "Designers", wait: 10
    end
  end

  private
    def row_for(room)
      find("#sidebar a[data-room-id='#{room.id}']")
    end

    def open_room_menu(room)
      row_for(room).right_click
      assert_selector "#room-menu:not([hidden])", wait: 5
    end

    def favorite_names
      page.evaluate_script(<<~JS)
        Array.from(document.querySelectorAll("#favorite_rooms .sidebar-item__label, #favorite_rooms .direct__author"))
          .map(node => node.textContent.trim())
      JS
    end

    def within_category(name, &block)
      section = find("#sidebar .sidebar-section--category", text: /#{name}/i)
      within(section, &block)
    end

    # A synthetic HTML5 drag: Selenium's own drag-and-drop never fires
    # real dragstart/drop, so the drop position is driven with dispatched
    # DragEvents against the same handlers the browser would call.
    def drag_favorite_to_top(room)
      page.execute_script(<<~JS, room.id)
        const roomId = String(arguments[0]);
        const list = document.getElementById("favorite_rooms");
        const row = list.querySelector(`a[data-room-id="${roomId}"]`);
        const first = list.querySelector("a[data-room-id]");
        const y = first.getBoundingClientRect().top + 2;

        row.dispatchEvent(new DragEvent("dragstart", { bubbles: true, cancelable: true, dataTransfer: new DataTransfer() }));
        list.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, clientY: y, dataTransfer: new DataTransfer() }));
        list.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, clientY: y, dataTransfer: new DataTransfer() }));
        row.dispatchEvent(new DragEvent("dragend", { bubbles: true, cancelable: true }));
      JS
    end
end
