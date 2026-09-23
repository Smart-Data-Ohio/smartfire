require "test_helper"

class RoomsUnreadDividerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
    @membership = users(:david).memberships.find_by!(room: @room)
  end

  test "no divider when everything is read" do
    @membership.read

    get room_url(@room)
    assert_response :success
    assert_no_match "unread-divider", @response.body
    assert_no_match "jump-to-unread", @response.body
  end

  test "divider renders above the first unread message" do
    first, second = @room.root_messages.ordered.first(2)
    @membership.update!(unread_at: second.created_at, last_read_message_id: first.id)

    get room_url(@room)
    assert_response :success
    assert_match "unread-divider", @response.body

    divider_at = @response.body.index("unread-divider")
    second_at = @response.body.index("data-message-id=\"#{second.id}\"")
    third_at = @response.body.index("data-message-id=\"#{@room.root_messages.ordered.third.id}\"")
    assert divider_at < second_at, "expected the divider above the first unread message"
    assert second_at < third_at
  end

  test "few unread keep the bottom scroll, many unread scroll to the divider" do
    first = @room.root_messages.ordered.first
    @membership.update!(unread_at: 1.hour.ago, last_read_message_id: first.id)

    get room_url(@room)
    assert_response :success
    assert_no_match "messages-scroll-to-divider-value", @response.body

    6.times do |i|
      @room.root_messages.create!(creator: users(:jason), body: "Catch-up #{i}", client_message_id: "catch-up-#{i}")
    end

    get room_url(@room)
    assert_response :success
    assert_match "messages-scroll-to-divider-value=\"true\"", @response.body
  end

  test "a first unread off the last page keeps the page and links the pill to it" do
    first = @room.root_messages.ordered.first
    @membership.update!(unread_at: 1.hour.ago, last_read_message_id: first.id)
    first_unread = @membership.first_unread_message

    (Message::PAGE_SIZE + 5).times do |i|
      travel 1.second do
        @room.root_messages.create!(creator: users(:jason), body: "Overflow #{i}", client_message_id: "overflow-#{i}")
      end
    end

    get room_url(@room)
    assert_response :success
    assert_no_match "unread-divider", @response.body
    assert_select "a#jump-to-unread[href=?]", room_path(@room, message_id: first_unread.id)
  end

  test "an anchored message keeps its own page with the divider when visible" do
    messages = @room.root_messages.ordered.to_a
    @membership.update!(unread_at: messages.second.created_at, last_read_message_id: messages.first.id)

    get room_url(@room, message_id: messages.third.id)
    assert_response :success
    assert_match "unread-divider", @response.body
  end

  test "watching live advances the pointer and returning shows no divider" do
    @membership.read
    @membership.update!(connected_at: Time.current, connections: 1)

    live = @room.root_messages.create!(creator: users(:jason), body: "Seen live", client_message_id: "live-seen-1")

    assert_not @membership.reload.unread?
    assert_equal live.id, @membership.last_read_message_id

    get room_url(@room)
    assert_response :success
    assert_no_match "unread-divider", @response.body
    assert_no_match "jump-to-unread", @response.body
  end

  test "messages watched live stay read when later messages go unread" do
    @membership.read
    @membership.update!(connected_at: Time.current, connections: 1)

    live = @room.root_messages.create!(creator: users(:jason), body: "Seen live", client_message_id: "live-seen-2")
    assert_equal live.id, @membership.reload.last_read_message_id

    @membership.update_columns(connected_at: nil, connections: 0)
    away = @room.root_messages.create!(creator: users(:jason), body: "While away", client_message_id: "live-away-1")

    get room_url(@room)
    assert_response :success
    assert_match "unread-divider", @response.body

    body = @response.body
    divider_at = body.index("unread-divider")
    live_at = body.index("data-message-id=\"#{live.id}\"")
    away_at = body.index("data-message-id=\"#{away.id}\"")
    assert live_at < divider_at, "expected the live-seen message above the divider"
    assert divider_at < away_at, "expected the divider above the while-away message"
    assert_equal 1, @membership.reload.unread_count
  end

  test "your own post never counts as unread" do
    @membership.read
    @membership.update_columns(connected_at: nil, connections: 0)

    mine = @room.root_messages.create!(creator: users(:david), body: "Mine", client_message_id: "own-post-1")

    assert_not @membership.reload.unread?
    assert_equal mine.id, @membership.last_read_message_id

    get room_url(@room)
    assert_response :success
    assert_no_match "unread-divider", @response.body

    theirs = @room.root_messages.create!(creator: users(:jason), body: "Theirs", client_message_id: "own-post-2")

    get room_url(@room)
    assert_response :success

    body = @response.body
    divider_at = body.index("unread-divider")
    mine_at = body.index("data-message-id=\"#{mine.id}\"")
    theirs_at = body.index("data-message-id=\"#{theirs.id}\"")
    assert mine_at < divider_at, "expected the own post above the divider"
    assert divider_at < theirs_at, "expected the divider above the other post"
    assert_equal 1, @membership.reload.unread_count
  end
end
