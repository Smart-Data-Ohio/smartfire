require "test_helper"

# ChannelThread#messages_count backs the room's "N replies" thread
# indicator. It counts the thread's finished, non-system messages and is
# recomputed from the rows on every change that can move it.
class ChannelThreadMessagesCountTest < ActiveSupport::TestCase
  include ActionCable::TestHelper
  include Turbo::Broadcastable::TestHelper

  setup do
    @room = rooms(:designers)
    @parent = messages(:third)
    @thread = ChannelThread.create!(room: @room, creator: users(:jz), parent_message: @parent, name: "Counted")
  end

  test "a new thread starts at zero" do
    assert_equal 0, @thread.reload.messages_count
  end

  test "posting a reply counts it and bumps the parent message" do
    stamp = @parent.reload.updated_at

    travel 1.minute do
      post_reply("First")
    end

    assert_equal 1, @thread.reload.messages_count
    assert_operator @parent.reload.updated_at, :>, stamp
  end

  test "the counter refresh leaves the thread's own updated_at alone" do
    reply = post_reply("First")
    stamp = @thread.reload.updated_at

    travel 1.minute do
      reply.destroy!
    end

    assert_equal 0, @thread.reload.messages_count
    assert_equal stamp, @thread.updated_at
  end

  test "deleting an older reply lowers the count" do
    older = post_reply("Older")
    post_reply("Newer")
    assert_equal 2, @thread.reload.messages_count

    older.destroy!

    assert_equal 1, @thread.reload.messages_count
  end

  test "system notes never count" do
    post_reply("Real")
    Message.create!(room: @room, thread: @thread, creator: users(:jz), system_note: true, markdown_source: "pinned a message")

    assert_equal 1, @thread.reload.messages_count
  end

  test "a streaming reply counts only once it finalizes" do
    agent_user = users(:bender)
    @room.memberships.grant_to(agent_user)
    stream = @thread.post_message!(creator: agent_user, attributes: {
      markdown_source: "Thinking", client_message_id: "counter-stream", streaming: true })

    assert_equal 0, @thread.reload.messages_count

    assert stream.finalize_stream!
    assert_equal 1, @thread.reload.messages_count
  end

  test "a quietly finalized stream counts too" do
    agent_user = users(:bender)
    @room.memberships.grant_to(agent_user)
    stream = @thread.post_message!(creator: agent_user, attributes: {
      markdown_source: "Thinking", client_message_id: "counter-quiet-stream", streaming: true })

    assert stream.finalize_stream_quietly!
    assert_equal 1, @thread.reload.messages_count
  end

  test "deleting a streaming reply leaves the count alone" do
    post_reply("Done")
    agent_user = users(:bender)
    @room.memberships.grant_to(agent_user)
    stream = @thread.post_message!(creator: agent_user, attributes: {
      markdown_source: "Thinking", client_message_id: "counter-dropped-stream", streaming: true })

    stream.destroy!

    assert_equal 1, @thread.reload.messages_count
  end

  test "the count heals from the rows after a missed refresh" do
    post_reply("One")
    ChannelThread.where(id: @thread.id).update_all(messages_count: 42)

    post_reply("Two")

    assert_equal 2, @thread.reload.messages_count
  end

  test "destroying the thread with its replies succeeds and stamps the parent" do
    post_reply("One")
    post_reply("Two")
    stamp = @parent.reload.updated_at

    travel 1.minute do
      @thread.reload.destroy!
    end

    assert_nil ChannelThread.find_by(id: @thread.id)
    assert_operator @parent.reload.updated_at, :>, stamp
  end

  test "a posted reply broadcasts the parent's indicator with the new count" do
    streams = capture_room_streams do
      post_reply("First")
    end

    indicator = indicator_stream(streams)
    assert indicator, "expected a thread indicator replace on the room stream"
    assert_includes indicator.to_html, "1 reply"
    assert_not indicator.at_css("button[hidden]")
  end

  test "a deleted reply broadcasts the lowered count" do
    older = post_reply("Older")
    post_reply("Newer")

    streams = capture_room_streams do
      older.destroy!
    end

    indicator = indicator_stream(streams)
    assert indicator, "expected a thread indicator replace on the room stream"
    assert_includes indicator.to_html, "1 reply"
  end

  test "deleting the last reply broadcasts a hidden indicator" do
    reply = post_reply("Only")

    streams = capture_room_streams do
      reply.destroy!
    end

    assert indicator_stream(streams).at_css("button[hidden]")
  end

  test "a finalized thread stream broadcasts the parent's indicator" do
    agent_user = users(:bender)
    @room.memberships.grant_to(agent_user)
    stream = @thread.post_message!(creator: agent_user, attributes: {
      markdown_source: "Thinking", client_message_id: "counter-broadcast-stream", streaming: true })

    streams = capture_room_streams do
      stream.finalize_stream!
    end

    assert_includes indicator_stream(streams).to_html, "1 reply"
  end

  test "deleting the thread broadcasts a hidden indicator" do
    post_reply("One")

    streams = capture_room_streams do
      @thread.reload.destroy!
    end

    indicator = indicator_stream(streams)
    assert indicator, "expected a thread indicator replace on the room stream"
    assert indicator.at_css("button[hidden]")
  end

  test "a failing indicator broadcast skips no finalize side effect" do
    stream = start_agent_stream("counter-raising-stream", "hovercraft")

    stream.stubs(:broadcast_thread_indicator).raises(Redis::CannotConnectError, "cable down")
    streams = nil
    assert_enqueued_with job: ChannelThread::PushMessageJob do
      streams = capture_thread_streams { assert stream.finalize_stream! }
    end

    assert_equal 1, @thread.reload.messages_count
    assert_equal [ stream ], @room.messages.search("hovercraft")
    assert streams.any? { |element| element["action"] == "replace" }, "expected the stream's final replace"
  end

  test "a failing indicator broadcast still ends a quiet finalize" do
    stream = start_agent_stream("counter-raising-quiet-stream", "Thinking")

    stream.stubs(:broadcast_thread_indicator).raises(Redis::CannotConnectError, "cable down")
    streams = capture_thread_streams { assert stream.finalize_stream_quietly! }

    assert_equal 1, @thread.reload.messages_count
    assert streams.any? { |element| element["action"] == "replace" }, "expected the stream's final replace"
  end

  test "a user's hard destroy recounts the threads their replies leave" do
    post_reply("Stays")
    @room.memberships.grant_to(users(:kevin))
    @thread.post_message!(creator: users(:kevin), attributes: { markdown_source: "Goes with Kevin" })
    assert_equal 2, @thread.reload.messages_count

    users(:kevin).destroy!

    assert_equal 1, @thread.reload.messages_count
  end

  test "a system note in the thread broadcasts nothing" do
    post_reply("Real")

    streams = capture_room_streams do
      Message.create!(room: @room, thread: @thread, creator: users(:jz), system_note: true, markdown_source: "note")
    end

    assert_nil indicator_stream(streams)
  end

  private
    def start_agent_stream(client_message_id, text)
      @room.memberships.grant_to(users(:bender))
      @thread.post_message!(creator: users(:bender), attributes: {
        markdown_source: text, client_message_id:, streaming: true })
    end

    def capture_thread_streams(&block)
      ActionCable.server.pubsub.clear
      capture_turbo_stream_broadcasts([ @thread, :messages ], &block)
    end

    def post_reply(text)
      @thread.post_message!(creator: users(:jz), attributes: { markdown_source: text })
    end

    # Only what the block broadcasts: earlier setup posts broadcast too.
    def capture_room_streams(&block)
      ActionCable.server.pubsub.clear
      capture_turbo_stream_broadcasts([ @room, :messages ], &block)
    end

    def indicator_stream(streams)
      target = ActionView::RecordIdentifier.dom_id(@parent, :thread_indicator)
      streams.find { |stream| stream["action"] == "replace" && stream["target"] == target }
    end
end
