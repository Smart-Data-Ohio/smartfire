require "test_helper"

class MessageStreamingTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
    @bot = users(:bender)
    @watcher = create_agent_in(@room, name: "Stream Watcher")
    @legacy = User.create_bot!(name: "Legacy Stream", webhook_url: "https://example.test/legacy-stream")
    @room.memberships.grant_to(@legacy)
    @david_membership = Membership.find_by!(room: @room, user: users(:david))
  end

  test "a streaming create fires no side effects" do
    assert_no_enqueued_jobs do
      @message = @room.root_messages.create!(creator: @bot, streaming: true,
        markdown_source: "Hey @[David] and @[Stream Watcher] and @[Legacy Stream] hovercraft",
        client_message_id: "stream-quiet")
    end

    assert_empty ActivityItem.where(source: @message)
    assert_empty @agent.agent_events.where(message_id: @message.id)
    assert_empty @watcher.agent_events.where(message_id: @message.id)
    assert_equal [], @room.messages.search("hovercraft")
    assert_nil @david_membership.reload.unread_at
  end

  test "stream start broadcasts the append without the unread broadcast" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Starting", client_message_id: "stream-start")

    message.broadcast_stream_start

    stream = [ @room.to_gid_param, :messages ].join(":")
    appends = ActionCable.server.pubsub.broadcasts(stream)
    assert_equal 1, appends.size
    assert_includes appends.sole, ActionView::RecordIdentifier.dom_id(message)
    assert_empty ActionCable.server.pubsub.broadcasts(UnreadRoomsChannel.stream_name_for(users(:david).id))
    assert_empty ActionCable.server.pubsub.broadcasts(UnreadRoomsChannel.stream_name_for(users(:jason).id))
  end

  test "finalize fires every side effect exactly once" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Hey @[David] and @[Stream Watcher] and @[Legacy Stream] hovercraft",
      client_message_id: "stream-final")

    assert_difference -> { ActivityItem.where(source: message).count }, 1 do
      assert_difference -> { @agent.agent_events.where(event_type: "posted").count }, 1 do
        assert_difference -> { @watcher.agent_events.where(event_type: "mention").count }, 1 do
          assert_enqueued_jobs 3 do
            assert message.finalize_stream!
          end
        end
      end
    end

    assert_not message.reload.streaming?
    assert_equal [ message ], @room.messages.search("hovercraft")
    assert_equal message.created_at, @david_membership.reload.unread_at
    assert_enqueued_with job: Room::PushMessageJob
    assert_enqueued_with job: Agent::DeliveryJob
    assert_enqueued_with job: Bot::WebhookJob

    stream = [ @room.to_gid_param, :messages ].join(":")
    replaces = ActionCable.server.pubsub.broadcasts(stream)
    assert_equal 1, replaces.size
    assert_includes replaces.sole, ActionView::RecordIdentifier.dom_id(message)

    david_badges = ActionCable.server.pubsub.broadcasts(UnreadRoomsChannel.stream_name_for(users(:david).id))
    assert_equal 1, david_badges.size
    assert_equal @room.id, JSON.parse(david_badges.sole)["roomId"]

    assert_no_difference [ "ActivityItem.count", "AgentEvent.count" ] do
      assert_no_enqueued_jobs do
        assert_not message.finalize_stream!
      end
    end
  end

  test "finalizing a thread stream sends no unread-room broadcast" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Thread badges")
    ThreadMembership.join!(thread, users(:david))
    message = thread.post_message!(creator: @bot, attributes: {
      markdown_source: "Replying", client_message_id: "stream-thread-badge", streaming: true })

    message.finalize_stream!

    assert_not message.reload.streaming?
    assert_empty ActionCable.server.pubsub.broadcasts(UnreadRoomsChannel.stream_name_for(users(:david).id))
    assert_empty ActionCable.server.pubsub.broadcasts(UnreadRoomsChannel.stream_name_for(users(:jason).id))
  end

  test "appends re-render without firing side effects" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Hey @[David]", client_message_id: "stream-append")

    assert_no_difference [ "ActivityItem.count", "AgentEvent.count" ] do
      assert_no_enqueued_jobs do
        message.update!(markdown_source: "Hey @[David] hovercraft")
        message.update!(markdown_source: "Hey @[David] hovercraft eels")
      end
    end

    assert_equal [], @room.messages.search("hovercraft")

    message.finalize_stream!

    assert_equal [ message ], @room.messages.search("hovercraft")
    assert_equal 1, ActivityItem.where(source: message).count
  end

  test "stream broadcasts coalesce to about four per second" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Ticking", client_message_id: "stream-throttle")
    stream = [ message.message_stream_target.to_gid_param, :messages ].join(":")

    travel_to Time.current do
      before = ActionCable.server.pubsub.broadcasts(stream).size

      10.times { message.broadcast_stream_update }

      assert_equal before + 1, ActionCable.server.pubsub.broadcasts(stream).size
    end

    travel 1.second do
      assert message.broadcast_stream_update
    end

    assert_equal 2, ActionCable.server.pubsub.broadcasts(stream).size
  end

  test "overdue streams finalize from the sweep, fresh ones stay streaming" do
    fresh = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Fresh", client_message_id: "stream-fresh")
    old = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Old hovercraft", client_message_id: "stream-old")
    old.update_columns(created_at: 11.minutes.ago, streaming_updated_at: 11.minutes.ago)

    Message.finalize_overdue_streams!

    assert_not old.reload.streaming?
    assert fresh.reload.streaming?
    assert_equal [ old ], @room.messages.search("hovercraft")
    assert_equal 1, @agent.agent_events.where(event_type: "posted", message_id: old.id).count
  end

  test "the sweep keys on inactivity, not creation" do
    active = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Still going", client_message_id: "stream-active")
    active.update_columns(created_at: 20.minutes.ago, streaming_updated_at: 1.minute.ago)
    idle = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Gone quiet", client_message_id: "stream-idle")
    idle.update_columns(created_at: 20.minutes.ago, streaming_updated_at: 11.minutes.ago)

    Message.finalize_overdue_streams!

    assert_predicate active.reload, :streaming?
    assert_not idle.reload.streaming?
  end

  test "starting and appending stamp the stream's last activity" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Draft", client_message_id: "stream-stamp")

    assert_in_delta Time.current, message.reload.streaming_updated_at, 5

    travel 5.minutes do
      result = Agents::Streaming.update(agent: @agent, id: message.id, append: " more")

      assert result.ok?
      assert_in_delta Time.current, message.reload.streaming_updated_at, 5
    end

    travel 5.minutes do
      result = Agents::Streaming.update(agent: @agent, id: message.id, markdown_source: "Replaced")

      assert result.ok?
      assert_in_delta Time.current, message.reload.streaming_updated_at, 5
    end
  end

  test "an append after a finalize is refused and leaves the final alone" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Draft", client_message_id: "stream-lost-race")

    assert message.finalize_stream!

    result = Agents::Streaming.update(agent: @agent, id: message.id, append: "Late")
    assert_not result.ok?
    assert_equal :unprocessable_entity, result.status
    assert_equal "Message is not streaming", result.error

    result = Agents::Streaming.update(agent: @agent, id: message.id, markdown_source: "Replaced late")
    assert_not result.ok?
    assert_equal :unprocessable_entity, result.status

    assert_not message.reload.streaming?
    assert_equal "Draft", message.markdown_source
  end

  test "a finalized stream can never resume" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Draft", client_message_id: "stream-no-resume")

    assert message.finalize_stream!

    message.streaming = true

    assert_not message.valid?
    assert_equal [ "cannot resume once finalized" ], message.errors[:streaming]
  end

  test "the overdue sweep query uses the streaming activity index" do
    plan = ActiveRecord::Base.connection.execute(
      "EXPLAIN QUERY PLAN #{Message.overdue_streams.to_sql}"
    ).map { |row| row["detail"] }.join("\n")

    assert_match(/SEARCH messages USING INDEX index_messages_on_streaming_updated_at/, plan)
  end

  test "the sweep skips streams in locked threads until unlock" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Locked stream")
    ThreadMembership.join!(thread, users(:david))
    message = thread.post_message!(creator: @bot, attributes: {
      markdown_source: "Waiting", client_message_id: "stream-locked-sweep", streaming: true })
    message.update_columns(created_at: 11.minutes.ago, streaming_updated_at: 11.minutes.ago)
    thread.lock_conversation!

    Message.finalize_overdue_streams!

    assert_predicate message.reload, :streaming?
    assert_empty @agent.agent_events.where(message_id: message.id)

    thread.unlock_conversation!
    Message.finalize_overdue_streams!

    assert_not message.reload.streaming?
    assert_equal 1, @agent.agent_events.where(event_type: "posted", message_id: message.id).count
  end

  test "finalize skips side effects for a suspended agent" do
    @agent.suspend!
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Hey @[David] hovercraft", client_message_id: "stream-suspended")

    assert_no_enqueued_jobs do
      assert message.finalize_stream!
    end

    assert_not message.reload.streaming?
    assert_empty ActivityItem.where(source: message)
    assert_empty @agent.agent_events.where(message_id: message.id)
    assert_equal [], @room.messages.search("hovercraft")
    assert_nil @david_membership.reload.unread_at
  end

  test "the sweep finalizes a suspended agent's streams quietly" do
    @agent.suspend!
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Gone quiet hovercraft", client_message_id: "stream-sweep-quiet")
    message.update_columns(created_at: 11.minutes.ago, streaming_updated_at: 11.minutes.ago)

    assert_no_enqueued_jobs do
      Message.finalize_overdue_streams!
    end

    assert_not message.reload.streaming?
    assert_empty @agent.agent_events.where(message_id: message.id)
    assert_equal [], @room.messages.search("hovercraft")
  end

  test "finalize skips side effects for a deactivated agent" do
    @bot.update!(status: :deactivated)
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Hey @[David] hovercraft", client_message_id: "stream-deactivated")

    assert_no_enqueued_jobs do
      assert message.finalize_stream!
    end

    assert_not message.reload.streaming?
    assert_empty ActivityItem.where(source: message)
    assert_empty @agent.agent_events.where(message_id: message.id)
    assert_equal [], @room.messages.search("hovercraft")
  end

  test "finalize clears the streaming agent's working presence" do
    @agent.set_working_presence!("Thinking…")
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "Almost done", client_message_id: "stream-presence")

    message.finalize_stream!

    assert_nil @agent.reload.working_presence
  end

  test "a stream may start empty and require no body until finalize" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "", client_message_id: "stream-empty")

    assert_predicate message, :persisted?

    message.finalize_stream!

    assert_not message.reload.streaming?
  end

  test "link references sync at finalize, not while streaming" do
    message = @room.root_messages.create!(creator: @bot, streaming: true,
      markdown_source: "See https://example.test/some/page", client_message_id: "stream-links")

    assert_empty message.link_embed_references.reload

    message.update!(markdown_source: "See https://example.test/some/page and more")

    assert_empty message.link_embed_references.reload

    message.finalize_stream!

    assert_equal 1, message.link_embed_references.reload.count
  end

  private
    def create_agent_in(room, name:)
      bot = User.create_bot!(name: name)
      agent = bot.create_agent!(kind: :workspace, owner: users(:david))
      room.memberships.grant_to(bot)
      agent
    end
end
