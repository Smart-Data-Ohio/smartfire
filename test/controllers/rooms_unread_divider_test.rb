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
end
