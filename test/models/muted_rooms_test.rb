require "test_helper"

class MutedRoomsTest < ActiveSupport::TestCase
  include ActiveJob::TestHelper

  setup do
    stub_web_push_dns_resolution
  end

  test "muted rooms go unread only when the member is mentioned" do
    memberships(:kevin_designers).update!(involvement: "muted")

    rooms(:designers).messages.create! body: "Hello all", client_message_id: "mute-plain", creator: users(:david)
    assert_not memberships(:kevin_designers).reload.unread?
    assert memberships(:jason_designers).reload.unread?

    rooms(:designers).messages.create!(
      body: "Hey #{mention_attachment_for(:kevin)}", client_message_id: "mute-mention", creator: users(:david)
    )
    assert memberships(:kevin_designers).reload.unread?
  end

  test "muted members are pushed only when mentioned" do
    memberships(:kevin_designers).update!(involvement: "muted")

    perform_enqueued_jobs only: Room::PushMessageJob do
      WebPush.expects(:payload_send).times(2)
      rooms(:designers).messages.create! body: "Hello all", client_message_id: "mute-push-plain", creator: users(:david)
    end
    wait_for_web_push_delivery_pool_tasks(2)

    perform_enqueued_jobs only: Room::PushMessageJob do
      WebPush.expects(:payload_send).times(3)
      rooms(:designers).messages.create!(
        body: "Hey #{mention_attachment_for(:kevin)}", client_message_id: "mute-push-mention", creator: users(:david)
      )
    end
    wait_for_web_push_delivery_pool_tasks(5)
  end

  test "muted members get no unread broadcast without a mention" do
    memberships(:kevin_designers).update!(involvement: "muted")
    stream = UnreadRoomsChannel.stream_name_for(users(:kevin).id)
    before = ActionCable.server.pubsub.broadcasts(stream).size

    rooms(:designers).messages.create!(
      body: "Hello all", client_message_id: "mute-broadcast-plain", creator: users(:david)
    ).broadcast_create
    assert_equal before, ActionCable.server.pubsub.broadcasts(stream).size

    rooms(:designers).messages.create!(
      body: "Hey #{mention_attachment_for(:kevin)}", client_message_id: "mute-broadcast-mention", creator: users(:david)
    ).broadcast_create
    assert_equal before + 1, ActionCable.server.pubsub.broadcasts(stream).size
  end

  test "muted members get no reply-author push" do
    room = rooms(:designers)
    memberships(:jason_designers).update!(involvement: "muted")
    source = room.root_messages.create!(creator: users(:jason), markdown_source: "Original", client_message_id: "mute-reply-source")
    clear_enqueued_jobs

    perform_enqueued_jobs only: Room::PushMessageJob do
      # jz follows everything; jason's muted reply notification stays silent.
      WebPush.expects(:payload_send).once
      room.root_messages.create!(
        creator: users(:david), markdown_source: "Reply with a ping", client_message_id: "mute-reply-on",
        reply_to_message: source, reply_notify_author: true
      )
    end
    wait_for_web_push_delivery_pool_tasks(1)
  end

  test "muted members still get mention inbox items but no replies" do
    memberships(:kevin_designers).update!(involvement: "muted")
    source = rooms(:designers).root_messages.create!(
      creator: users(:kevin), markdown_source: "Original", client_message_id: "mute-inbox-source"
    )

    assert_difference -> { ActivityItem.where(user: users(:kevin), event_type: "mention").count } do
      rooms(:designers).messages.create!(
        body: "Hey #{mention_attachment_for(:kevin)}", client_message_id: "mute-inbox-mention", creator: users(:david)
      )
    end

    assert_no_difference -> { ActivityItem.where(user: users(:kevin), event_type: "reply").count } do
      rooms(:designers).root_messages.create!(
        creator: users(:david), markdown_source: "Reply with a ping", client_message_id: "mute-inbox-reply",
        reply_to_message: source, reply_notify_author: true
      )
    end
  end

  test "unread broadcast ids cost one membership query without muted members" do
    message = rooms(:designers).messages.create!(
      body: "Hello all", client_message_id: "mute-query-plain", creator: users(:david)
    )

    assert_equal 1, count_queries { message.send(:unread_user_ids) }
    assert_equal rooms(:designers).memberships.pluck(:user_id).sort, message.send(:unread_user_ids).sort
  end

  test "unread broadcast ids filter muted members with one extra mention lookup" do
    memberships(:kevin_designers).update!(involvement: "muted")

    plain = rooms(:designers).messages.create!(
      body: "Hello all", client_message_id: "mute-query-plain-2", creator: users(:david)
    )
    # No mentions, so the empty mention lookup costs no query.
    assert_equal 1, count_queries { plain.send(:unread_user_ids) }
    assert_not_includes plain.send(:unread_user_ids), users(:kevin).id

    mentioned = rooms(:designers).messages.create!(
      body: "Hey #{mention_attachment_for(:kevin)}", client_message_id: "mute-query-mention", creator: users(:david)
    )
    assert_equal 2, count_queries { mentioned.send(:unread_user_ids) }
    assert_includes mentioned.send(:unread_user_ids), users(:kevin).id
  end

  private
    def count_queries
      count = 0
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 unless payload[:name] == "SCHEMA" || payload[:cached]
      end

      ActiveRecord::Base.connection_pool.clear_query_cache
      yield
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end
    def wait_for_web_push_delivery_pool_tasks(count)
      wait_for_pool_tasks(Rails.configuration.x.web_push_pool.delivery_pool, count)
    end

    def wait_for_pool_tasks(pool, count)
      start = Time.now
      timeout = 0.2
      while pool.completed_task_count < count
        raise "Timeout waiting for pool tasks to complete" if Time.now - start > timeout
        sleep timeout / 10.0
      end
    end
end
