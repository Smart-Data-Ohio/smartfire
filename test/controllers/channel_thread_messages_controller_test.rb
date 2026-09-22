require "test_helper"

class ChannelThreadMessagesControllerTest < ActionDispatch::IntegrationTest
  setup do
    host! "smartfire.test"
    @room = rooms(:designers)
    @thread = ChannelThread.create!(room: @room, creator: users(:jz), name: "Thread messages")
    ThreadMembership.join!(@thread, users(:jz))
    @message = @thread.post_message!(creator: users(:jz), attributes: { markdown_source: "Original", client_message_id: "thread-original" })
  end

  test "a post joins and reopens an unlocked closed thread atomically" do
    @thread.close!
    assert_not @thread.memberships.exists?(user: users(:kevin))
    sign_in :kevin

    post room_thread_messages_url(@room, @thread, format: :json), params: {
      message: { markdown_source: "Joined reply", client_message_id: "joined-reply" }
    }

    assert_response :created
    assert_predicate @thread.reload, :active?
    assert @thread.memberships.exists?(user: users(:kevin))
    assert_equal "Joined reply", @thread.messages.order(:id).last.plain_text_body
  end

  test "locked threads block every edit and post while delete stays author-or-admin" do
    @thread.lock_conversation!
    sign_in :jz

    patch room_thread_message_url(@room, @thread, @message, format: :json), params: { message: { markdown_source: "Edited" } }
    assert_response :forbidden
    assert_equal "Original", @message.reload.plain_text_body

    post room_thread_messages_url(@room, @thread, format: :json), params: { message: { markdown_source: "Blocked", client_message_id: "blocked-thread-post" } }
    assert_response :forbidden

    get actions_room_thread_message_url(@room, @thread, @message, format: :json)
    assert_response :success
    assert_equal "no-store", response.headers["Cache-Control"]
    assert_not response.parsed_body.dig("actions", "can_edit")

    sign_in :david
    patch room_thread_message_url(@room, @thread, @message, format: :json), params: { message: { markdown_source: "Admin blocked" } }
    assert_response :forbidden

    delete room_thread_message_url(@room, @thread, @message, format: :json)
    assert_response :no_content
  end

  test "thread unread state changes for every joined user regardless of preference" do
    nothing = users(:kevin)
    everything = users(:jason)
    ThreadMembership.join!(@thread, nothing).update!(involvement: "nothing")
    ThreadMembership.join!(@thread, everything).update!(involvement: "everything")

    @thread.post_message!(creator: users(:jz), attributes: { markdown_source: "Unread all", client_message_id: "unread-all" })

    assert @thread.memberships.find_by!(user: nothing).unread?
    assert @thread.memberships.find_by!(user: everything).unread?
  end

  test "editing a thread message to add a post URL broadcasts the new card" do
    sign_in :jz

    patch room_thread_message_url(@room, @thread, @message), params: {
      message: { markdown_source: "see https://x.com/jack/status/112233" }
    }

    assert_redirected_to room_thread_message_path(@room, @thread, @message)
    assert_equal [ "112233" ], @message.reload.twitter_posts.map(&:post_id)
    assert_rendered_turbo_stream_broadcast @thread, :messages, action: "replace", target: [ @message, :twitter_cards ] do
      assert_select ".x-post-card", text: /Loading post/
    end
  end

  test "editing a thread message to remove a post URL broadcasts an empty card container" do
    message = @thread.post_message!(
      creator: users(:jz),
      attributes: { markdown_source: "see https://x.com/jack/status/112234", client_message_id: "x-ref-thread-remove" }
    )
    assert_equal [ "112234" ], message.twitter_posts.map(&:post_id)
    sign_in :jz

    patch room_thread_message_url(@room, @thread, message), params: { message: { markdown_source: "never mind" } }

    assert_redirected_to room_thread_message_path(@room, @thread, message)
    assert_empty message.reload.twitter_posts
    assert_rendered_turbo_stream_broadcast @thread, :messages, action: "replace", target: [ message, :twitter_cards ] do
      assert_select ".x-post-card", count: 0
    end
  end

  test "destroy broadcasts tombstone updates and a thread summary refresh" do
    source = @thread.post_message!(
      creator: users(:jz),
      attributes: { markdown_source: "thread source", client_message_id: "thread-tombstone-source" }
    )
    reply = @thread.post_message!(
      creator: users(:jz),
      attributes: {
        markdown_source: "thread reply", reply_to_message_id: source.id, client_message_id: "thread-tombstone-reply"
      }
    )
    sign_in :jz

    delete room_thread_message_url(@room, @thread, source, format: :turbo_stream)

    assert_response :success
    assert_rendered_turbo_stream_broadcast @thread, :messages, action: "remove", target: source
    assert_rendered_turbo_stream_broadcast @thread, :messages, action: "replace", target: reply do
      assert_select ".message__reply-preview", text: /Replying to a deleted message/
    end

    refreshes = ActionCable.server.pubsub.broadcasts(UnreadThreadsChannel.stream_name_for(users(:jz).id))
      .map { |broadcast| JSON.parse(broadcast) }
      .select { |payload| payload["threadId"] == @thread.id && payload["refreshOnly"] == true }
    assert_equal 1, refreshes.size
  end

  test "edited thread messages show an edited marker" do
    sign_in :jz

    patch room_thread_message_url(@room, @thread, @message), params: { message: { markdown_source: "Edited here" } }

    assert_redirected_to room_thread_message_path(@room, @thread, @message)

    get room_thread_url(@room, @thread)
    assert_response :success
    assert_select ".message__edited", text: "(edited)"
  end

  test "nested HTML message URL redirects into the parent room shell" do
    sign_in :jz
    get room_thread_message_url(@room, @thread, @message)

    assert_redirected_to room_url(@room, thread: @thread.id, message_id: @message.id)
  end
end
