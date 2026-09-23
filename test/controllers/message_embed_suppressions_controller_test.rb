require "test_helper"

class MessageEmbedSuppressionsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:designers)
    @message = @room.messages.create!(
      creator: users(:david), client_message_id: "embed-suppress-target",
      markdown_source: "read https://example.com/suppress-me"
    )
  end

  test "the author removes embeds and clears both card containers" do
    sign_in :david

    assert_turbo_stream_broadcasts [ @room, :messages ], count: 2 do
      post room_message_embed_suppression_url(@room, @message), as: :turbo_stream
    end

    assert_response :success
    assert_predicate @message.reload, :embeds_suppressed?
    assert_equal "text/vnd.turbo-stream.html", response.media_type
    assert_select "turbo-stream[action=replace][target=?]", dom_id(@message, :linkedin_cards), count: 1
    assert_select "turbo-stream[action=replace][target=?]", dom_id(@message, :link_embed_cards), count: 1

    get room_url(@room)
    assert_response :success
    assert_select "##{dom_id(@message, :link_embed_cards)} .link-embed-card", count: 0
  end

  test "removal answers JSON as well" do
    sign_in :david

    post room_message_embed_suppression_url(@room, @message), as: :json

    assert_response :success
    assert_equal true, JSON.parse(response.body)["embeds_suppressed"]
    assert_predicate @message.reload, :embeds_suppressed?
  end

  test "another member cannot remove embeds" do
    sign_in :jz

    post room_message_embed_suppression_url(@room, @message), as: :json

    assert_response :forbidden
    assert_not @message.reload.embeds_suppressed?
  end

  test "an administrator who did not write the message cannot remove embeds" do
    @message.update!(creator: users(:jason))
    sign_in :david # administrator, but not the author

    post room_message_embed_suppression_url(@room, @message), as: :json

    assert_response :forbidden
    assert_not @message.reload.embeds_suppressed?
  end

  test "removal is refused on a locked thread message" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Locked")
    reply = thread.post_message!(creator: users(:david), attributes: {
      client_message_id: "embed-suppress-locked", markdown_source: "read https://example.com/locked"
    })
    thread.lock_conversation!
    sign_in :david

    post room_thread_message_embed_suppression_url(@room, thread, reply), as: :json

    assert_response :forbidden
    assert_not reply.reload.embeds_suppressed?
  end

  test "thread authors remove embeds through the thread route" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Open")
    reply = thread.post_message!(creator: users(:david), attributes: {
      client_message_id: "embed-suppress-thread", markdown_source: "read https://example.com/threaded"
    })
    sign_in :david

    post room_thread_message_embed_suppression_url(@room, thread, reply), as: :json

    assert_response :success
    assert_predicate reply.reload, :embeds_suppressed?
  end

  test "actions payload offers removal to the author while a renderable embed exists" do
    LinkEmbed.find_by!(normalized_url: "https://example.com/suppress-me").update!(
      title: "Suppress me", fetched_at: Time.current, fetch_error: nil,
      expires_at: 1.hour.from_now
    )
    sign_in :david
    get actions_room_message_url(@room, @message)

    assert_response :success
    actions = JSON.parse(response.body)["actions"]
    assert_equal true, actions["can_remove_embeds"]
    assert_equal room_message_embed_suppression_url(@room, @message), actions["suppress_embeds_url"]
  end

  test "actions payload withholds removal when no embed would render" do
    gated = @room.messages.create!(
      creator: users(:david), client_message_id: "embed-suppress-gated",
      markdown_source: "read https://example.com/gated-page"
    )
    LinkEmbed.find_by!(normalized_url: "https://example.com/gated-page").update!(
      fetched_at: Time.current, fetch_error: "No preview available for this link",
      expires_at: 1.hour.from_now
    )
    sign_in :david

    get actions_room_message_url(@room, gated)

    assert_response :success
    assert_equal false, JSON.parse(response.body)["actions"]["can_remove_embeds"]
  end

  test "actions payload offers removal for a LinkedIn chip with no fetched text" do
    chip = @room.messages.create!(
      creator: users(:david), client_message_id: "embed-suppress-chip",
      markdown_source: "see https://www.linkedin.com/posts/gated-chip-1"
    )
    sign_in :david

    get actions_room_message_url(@room, chip)

    assert_response :success
    assert_equal true, JSON.parse(response.body)["actions"]["can_remove_embeds"]
  end

  test "actions payload withholds removal from others and once removed" do
    sign_in :jason
    get actions_room_message_url(@room, @message)
    assert_equal false, JSON.parse(response.body)["actions"]["can_remove_embeds"]

    sign_in :david
    @message.update!(embeds_suppressed: true)
    get actions_room_message_url(@room, @message)
    assert_equal false, JSON.parse(response.body)["actions"]["can_remove_embeds"]
  end

  private
    def dom_id(record, prefix)
      ActionView::RecordIdentifier.dom_id(record, prefix)
    end
end
