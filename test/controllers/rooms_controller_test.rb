require "test_helper"

class RoomsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "index redirects to the user's last room" do
    get rooms_url
    assert_redirected_to room_url(users(:david).rooms.last)
  end

  test "show" do
    get room_url(users(:david).rooms.last)
    assert_response :success
  end

  test "shows records the last room visited in a cookie" do
    get room_url(users(:david).rooms.last)
    assert response.cookies[:last_room] = users(:david).rooms.last.id
  end

  test "show renders a link preview written by hand without its off-scheme image and link" do
    room = rooms(:watercooler)
    post room_messages_url(room, format: :turbo_stream), params: { message: {
      body: link_preview_body(href: "javascript:alert(1)", url: "data:image/svg+xml;base64,PHN2Zy8+"),
      client_message_id: "hand-written-preview" } }
    assert_response :success

    get room_url(room)

    assert_response :success
    assert_no_match /javascript:alert/, response.body
    assert_no_match /data:image\/svg/, response.body
    assert_match "Free cookies", response.body
  end

  test "show renders a link preview written by hand without its image pointed at this Smartfire" do
    room = rooms(:watercooler)
    own_url = room_url(room, host: "www.example.com")
    post room_messages_url(room, format: :turbo_stream), params: { message: {
      body: link_preview_body(href: own_url, url: own_url),
      client_message_id: "same-host-preview" } }
    assert_response :success

    get room_url(room)

    assert_response :success
    assert_no_match %r{<img src="#{Regexp.escape(own_url)}"}, response.body
    assert_no_match %r{<a rel="noreferrer" target="_blank" href="#{Regexp.escape(own_url)}"}, response.body
    assert_match "Free cookies", response.body
  end

  test "show renders an unfurled link preview" do
    room = rooms(:watercooler)
    post room_messages_url(room, format: :turbo_stream), params: { message: {
      body: link_preview_body(href: "https://example.com/page", url: "https://example.com/image.png"),
      client_message_id: "unfurled-preview" } }
    assert_response :success

    get room_url(room)

    assert_response :success
    assert_match %r{<img src="/embeds/image/[^"]+"}, response.body
    assert_not_includes response.body, "https://example.com/image.png"
    assert_match %r{href="https://example\.com/page"}, response.body
  end

  test "show renders the unread divider above the first unread message on the page" do
    room = rooms(:designers)
    membership = users(:david).memberships.find_by!(room: room)
    first_new = room.root_messages.create!(creator: users(:kevin), body: "First new", client_message_id: "show-divider-first")
    room.root_messages.create!(creator: users(:kevin), body: "Second new", client_message_id: "show-divider-second")
    membership.mark_unread_before(first_new)

    get room_url(room)

    assert_response :success
    assert_select "#unread-divider", text: /new messages/i
    assert response.body.index("unread-divider") < response.body.index("First new")
    assert_select "button#jump-to-unread", text: /jump to unread/i
  end

  test "show keeps the last page when the first unread fell off it and links the pill to it" do
    room = rooms(:designers)
    membership = users(:david).memberships.find_by!(room: room)
    first_new = room.root_messages.create!(creator: users(:kevin), body: "First unread off page", client_message_id: "show-offpage-first")
    (Message::PAGE_SIZE + 1).times do |index|
      room.root_messages.create!(creator: users(:kevin), body: "Later #{index}", client_message_id: "show-offpage-#{index}")
    end
    membership.mark_unread_before(first_new)

    get room_url(room)

    assert_response :success
    assert_select ".message", text: "First unread off page", count: 0
    assert_select "#unread-divider", count: 0
    assert_select "a#jump-to-unread[href=?]", room_path(room, message_id: first_new.id), text: /jump to unread/i
  end

  test "destroy removes the room from everyone and enqueues its deletion" do
    room = rooms(:designers)

    assert_turbo_stream_broadcasts :rooms, count: 1 do
      assert_enqueued_with(job: Room::DestroyJob, args: [ room.id ]) do
        delete room_url(room)
      end
    end

    assert_redirected_to root_url
    assert_predicate room.reload, :deleted?
    assert_empty room.memberships
  end

  test "destroy stamps the sweep claim" do
    room = rooms(:designers)

    delete room_url(room)

    assert_redirected_to root_url
    assert_not_nil room.reload.destroy_enqueued_at
  end

  test "destroy succeeds when the queue is down and the sweep recovers the room" do
    room = rooms(:designers)
    Room::DestroyJob.stubs(:perform_later).raises(Redis::BaseConnectionError, "Redis down")

    delete room_url(room)

    assert_redirected_to root_url
    assert_predicate room.reload, :deleted?
    assert_nil room.destroy_enqueued_at

    Room::DestroyJob.unstub(:perform_later)
    room.update_columns(deleted_at: 11.minutes.ago)
    assert_enqueued_with(job: Room::DestroyJob, args: [ room.id ]) do
      Room::DestroyJob.reenqueue_stuck!
    end
  end

  test "destroyed room is inaccessible while deletion is pending" do
    room = rooms(:designers)
    delete room_url(room)

    get room_url(room)
    assert_redirected_to root_url
  end

  test "destroy finishes through the enqueued job" do
    room = rooms(:designers)
    delete room_url(room)

    assert_difference -> { Room.count }, -1 do
      perform_enqueued_jobs
    end
  end

  test "destroy only allowed for creators or those who can administer" do
    sign_in :jz

    assert_no_enqueued_jobs do
      delete room_url(rooms(:designers))
      assert_response :forbidden
    end
    assert_not_predicate rooms(:designers).reload, :deleted?

    rooms(:designers).update! creator: users(:jz)

    assert_enqueued_with(job: Room::DestroyJob) do
      delete room_url(rooms(:designers))
    end
    assert_predicate rooms(:designers).reload, :deleted?
  end

  private
    def link_preview_body(href:, url:)
      %(<div><action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" ) +
        %(href="#{href}" url="#{url}" filename="Free cookies" caption="Cookies here"></action-text-attachment></div>)
    end
end
