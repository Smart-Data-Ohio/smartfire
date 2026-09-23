require "test_helper"

class Rooms::RefreshesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "refresh includes new messages since the last known" do
    travel_to 1.day.ago do
      @old_message = rooms(:hq).messages.create!(creator: users(:jason), body: "Old message", client_message_id: "old")
    end

    travel_to 1.minute.ago do
      @new_message = rooms(:hq).messages.create!(creator: users(:jason), body: "New message", client_message_id: "new")
      @old_message.touch
    end

    get room_refresh_url(rooms(:hq), format: :turbo_stream), params: { since: 10.minutes.ago.to_fs(:epoch) }

    assert_response :success

    assert_select "turbo-stream[action='append']" do
      assert_select "#" + dom_id(@new_message)
      assert_select "template", count: 1
    end

    assert_select "turbo-stream[action='replace']" do
      assert_select "#" + dom_id(@old_message)
      assert_select "template", count: 1
    end
  end

  test "refresh includes pin and unpin changes since the last sync" do
    room = rooms(:hq)
    pinned = room.messages.create!(creator: users(:david), body: "Stays pinned", client_message_id: "refresh-pin")
    unpinned = room.messages.create!(creator: users(:david), body: "Gets unpinned", client_message_id: "refresh-unpin")
    MessagePin.pin!(message: unpinned, pinner: users(:david))

    since = Time.current

    travel 1.minute do
      MessagePin.pin!(message: pinned, pinner: users(:david))
      unpinned.message_pins.sole.unpin!
    end

    get room_refresh_url(room, format: :turbo_stream), params: { since: since.to_fs(:epoch) }

    assert_response :success

    assert_select "turbo-stream[action='replace'][target='#{dom_id(pinned)}']" do
      assert_select "##{dom_id(pinned, :pin_badge)}:not([hidden])", 1
    end
    assert_select "turbo-stream[action='replace'][target='#{dom_id(unpinned)}']" do
      assert_select "##{dom_id(unpinned, :pin_badge)}[hidden]", 1
    end
    assert_select "turbo-stream[action='replace'][target='#{dom_id(room, :pins_count)}']", text: "1"
    assert_select "turbo-stream[action='replace'][target='#{dom_id(room, :pins_list)}']" do
      assert_select ".pins-panel__excerpt", text: /Stays pinned/
    end
  end

  test "refreshing a room the user no longer belongs to is a quiet 404" do
    get room_refresh_url(rooms(:watercooler), format: :turbo_stream), params: { since: 0 }
    assert_response :success

    users(:david).memberships.find_by!(room: rooms(:watercooler)).destroy!

    get room_refresh_url(rooms(:watercooler), format: :turbo_stream), params: { since: 0 }
    assert_response :not_found
  end
end
