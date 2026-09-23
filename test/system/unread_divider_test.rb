require "application_system_test_case"

class UnreadDividerTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
    @designers = rooms(:designers)
  end

  test "few unread render the divider above the first new message and keep the bottom scroll" do
    join_room @designers
    expire_connection
    join_room rooms(:hq)

    first_new = @designers.root_messages.create!(creator: users(:kevin), body: "Fresh one", client_message_id: "divider-few-1")
    @designers.root_messages.create!(creator: users(:kevin), body: "Fresh two", client_message_id: "divider-few-2")

    join_room @designers

    assert_selector "#unread-divider", text: /new messages/i, wait: 5
    assert_divider_above first_new
    assert_at_bottom
  end

  test "many unread scroll the room to the divider" do
    join_room @designers
    expire_connection
    join_room rooms(:hq)

    7.times do |i|
      @designers.root_messages.create!(creator: users(:kevin), body: "Catch-up #{i}", client_message_id: "divider-many-#{i}")
    end

    join_room @designers

    assert_selector "#unread-divider", wait: 5
    geometry = page.evaluate_script(<<~JS)
      (() => {
        const list = document.querySelector(".messages");
        const divider = document.getElementById("unread-divider");
        const listRect = list.getBoundingClientRect();
        const dividerRect = divider.getBoundingClientRect();
        return { top: dividerRect.top - listRect.top, listHeight: listRect.height };
      })()
    JS
    assert geometry["top"] < geometry["listHeight"] / 2,
      "expected the divider near the top, got #{geometry.inspect}"
  end

  test "the jump pill shows while the divider is off-screen and returns to it" do
    join_room @designers
    expire_connection
    join_room rooms(:hq)

    # Enough two-line messages to overflow the list, so the divider can
    # genuinely scroll off-screen.
    25.times do |i|
      @designers.root_messages.create!(creator: users(:kevin), body: "Pill #{i}\nsecond line", client_message_id: "divider-pill-#{i}")
    end

    join_room @designers
    assert_selector "#unread-divider", wait: 5

    page.evaluate_script("document.querySelector('.messages').scrollTop = document.querySelector('.messages').scrollHeight")
    assert_selector "#jump-to-unread", visible: true, wait: 5

    click_on "Jump to unread"

    # The pill hides exactly while the divider intersects the list,
    # so its disappearance is the jump landing.
    assert_no_selector "#jump-to-unread", visible: true, wait: 5
    assert_selector "#unread-divider"
  end

  test "mark unread from the message menu points the divider at that message" do
    join_room @designers

    target = @designers.root_messages.ordered.second
    open_message_menu target
    click_on "Mark unread"
    assert_room_unread @designers

    visit room_path(@designers)
    assert_selector "#unread-divider", wait: 5
    assert_divider_above target
  end

  private
    # Joining a room connects its membership for 60 seconds, during which
    # the server skips it when marking unread. Expire that grace period
    # so the messages below persist server-side.
    def expire_connection
      users(:jz).memberships.find_by!(room: @designers).update_columns(connected_at: nil, connections: 0)
    end

    def assert_divider_above(message)
      divider_message = page.evaluate_script(<<~JS)
        document.getElementById("unread-divider").nextElementSibling?.dataset.messageId
      JS
      assert_equal message.id.to_s, divider_message
    end

    def assert_at_bottom
      distance = page.evaluate_script(<<~JS)
        (() => {
          const list = document.querySelector(".messages");
          return list.scrollHeight - list.scrollTop - list.clientHeight;
        })()
      JS
      assert distance < 100, "expected the list scrolled to the bottom, #{distance}px away"
    end
end
