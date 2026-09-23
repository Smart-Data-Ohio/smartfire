require "test_helper"

class Rooms::InvolvementsMuteTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
    @membership = users(:david).memberships.find_by!(room: @room)
  end

  test "muting clears the unread state" do
    @membership.update!(unread_at: 1.hour.ago, last_read_message_id: @room.root_messages.ordered.first.id)

    put room_involvement_url(@room), params: { involvement: "muted" }
    assert_redirected_to room_involvement_url(@room)

    assert_not @membership.reload.unread?
    assert_equal @room.root_messages.ordered.last.id, @membership.last_read_message_id
  end

  test "muting and unmuting replace the sidebar row in place" do
    assert_turbo_stream_broadcasts [ users(:david), :rooms ], count: 1 do
      put room_involvement_url(@room), params: { involvement: "muted" }
    end

    streams = capture_turbo_stream_broadcasts([ users(:david), :rooms ])
    assert_equal "replace", streams.first["action"]
    assert_equal dom_id(@room, :list), streams.first["target"]
    assert_match "muted", streams.first.to_html

    assert_turbo_stream_broadcasts [ users(:david), :rooms ], count: 2 do
      put room_involvement_url(@room), params: { involvement: "mentions" }
    end

    streams = capture_turbo_stream_broadcasts([ users(:david), :rooms ])
    assert_equal "replace", streams.last["action"]
    assert_equal dom_id(@room, :list), streams.last["target"]
  end

  test "muting a direct room replaces its row too" do
    dm = rooms(:david_and_jason)

    assert_turbo_stream_broadcasts [ users(:david), :rooms ], count: 1 do
      put room_involvement_url(dm), params: { involvement: "muted" }
    end

    streams = capture_turbo_stream_broadcasts([ users(:david), :rooms ])
    assert_equal "replace", streams.first["action"]
    assert_equal dom_id(dm, :list), streams.first["target"]
  end

  test "repeating the current level neither crashes nor broadcasts" do
    @membership.update!(involvement: "muted")

    assert_no_turbo_stream_broadcasts [ users(:david), :rooms ] do
      put room_involvement_url(@room), params: { involvement: "muted" }
      assert_redirected_to room_involvement_url(@room)
    end
  end

  test "the bell cycles through muted with its own icon" do
    @membership.update!(involvement: "everything")

    get room_involvement_url(@room)
    assert_response :success
    assert_match "notification-bell-everything", @response.body
    assert_match "involvement=muted", @response.body

    @membership.update!(involvement: "muted")

    get room_involvement_url(@room)
    assert_response :success
    assert_match "notification-bell-muted", @response.body
  end
end
