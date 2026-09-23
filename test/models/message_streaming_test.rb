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

    assert_no_difference [ "ActivityItem.count", "AgentEvent.count" ] do
      assert_no_enqueued_jobs do
        assert_not message.finalize_stream!
      end
    end
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
    old.update_column(:created_at, 11.minutes.ago)

    Message.finalize_overdue_streams!

    assert_not old.reload.streaming?
    assert fresh.reload.streaming?
    assert_equal [ old ], @room.messages.search("hovercraft")
    assert_equal 1, @agent.agent_events.where(event_type: "posted", message_id: old.id).count
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
