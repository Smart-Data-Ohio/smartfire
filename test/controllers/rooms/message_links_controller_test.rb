require "test_helper"

class Rooms::MessageLinksControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
    @other_room = rooms(:watercooler)
    @source = @other_room.messages.create!(
      body: "cross-room quoted words", client_message_id: "quote-source",
      creator: users(:jason)
    )
    @quote = @room.messages.create!(
      markdown_source: "look over here /rooms/#{@other_room.id}/@#{@source.id}",
      client_message_id: "quote-quoting", creator: users(:david)
    )
    @reference = @quote.message_references.first
  end

  test "a member of the source room sees the quote card" do
    get room_message_link_url(@room, @reference)

    assert_response :success
    assert_select "blockquote.message-quote", text: /cross-room quoted words/
    assert_select ".message-quote__author", text: "Jason"
    assert_select ".message-quote__room", text: /All Talk/
    assert_select "a", text: "Jump to message"
  end

  test "a non-member of the source room sees only the private chip" do
    sign_in :kevin

    get room_message_link_url(@room, @reference)

    assert_response :success
    assert_select ".message-quote-private", text: "Message in a private room"
    assert_not_includes response.body, "cross-room quoted words"
    assert_select ".message-quote", count: 0
  end

  test "a non-member of the quoting room gets nothing" do
    sign_in :kevin

    get room_message_link_url(rooms(:watercooler), @reference)

    assert_response :not_found
  end

  test "a reference from another room gets nothing" do
    other_quote = @other_room.messages.create!(
      markdown_source: "elsewhere /rooms/#{@other_room.id}/@#{@source.id}",
      client_message_id: "quote-elsewhere", creator: users(:david)
    )

    get room_message_link_url(@room, other_quote.message_references.first)

    assert_response :not_found
  end

  test "a same-room quote renders inline in the room" do
    same_source = @room.messages.create!(
      body: "same-room quoted words", client_message_id: "quote-same-source", creator: users(:jz)
    )
    @room.messages.create!(
      markdown_source: "right here /rooms/#{@room.id}/@#{same_source.id}",
      client_message_id: "quote-same", creator: users(:david)
    )

    get room_url(@room)

    assert_response :success
    assert_select "blockquote.message-quote", text: /same-room quoted words/
  end

  test "a cross-room quote renders a lazy frame in the room" do
    get room_url(@room)

    assert_response :success
    assert_select "turbo-frame.message-link-frame[src=?]", room_message_link_path(@room, @reference)
  end

  test "editing the source refreshes quoting cards over the stream" do
    sign_in :jason
    patch room_message_url(@other_room, @source),
      params: { message: { body: "cross-room quoted words, revised" } }

    assert_response :redirect
    assert_rendered_turbo_stream_broadcast @room, :messages,
      action: "replace", target: [ @quote, :message_link_cards ] do |stream|
      assert_select stream, "turbo-frame.message-link-frame"
    end
  end

  test "deleting the source clears quoting cards and busts their cache" do
    @quote.update_columns(updated_at: 1.day.ago)

    delete room_message_url(@other_room, @source, format: :turbo_stream)

    assert_response :success
    assert_empty MessageReference.where(id: @reference.id)
    assert @quote.reload.updated_at > 1.day.ago
    assert_rendered_turbo_stream_broadcast @room, :messages,
      action: "replace", target: [ @quote, :message_link_cards ]
  end

  test "editing a message to add a permalink replaces its own card container" do
    plain = @room.messages.create!(
      body: "nothing quoted yet", client_message_id: "quote-plain", creator: users(:david)
    )

    patch room_message_url(@room, plain),
      params: { message: { markdown_source: "now quoting /rooms/#{@room.id}/@#{@source.id}" } }

    assert_response :redirect
    assert_equal [ @source ], plain.reload.referenced_messages
    assert_rendered_turbo_stream_broadcast @room, :messages,
      action: "replace", target: [ plain, :message_link_cards ]
  end
end
