require "test_helper"

# Transactional tests pin one shared connection, so worker threads would
# join the test transaction and lose their after_create_commit callbacks.
# This class opts out: workers commit for real on independent
# connections, and the test deletes everything they wrote.
class AgentDeliveryConcurrencyTest < ActiveSupport::TestCase
  self.use_transactional_tests = false

  setup do
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
    @created_ids = []
  end

  teardown do
    AgentEvent.where(message_id: @created_ids).delete_all
    ActivityItem.where(source_type: "Message", source_id: @created_ids).delete_all
    ActionText::RichText.where(record_type: "Message", record_id: @created_ids).delete_all
    Message.where(id: @created_ids).delete_all
    @membership_state&.each { |id, unread_at| Membership.where(id: id).update_all(unread_at: unread_at) }
    Room.where(id: @room.id).update_all(updated_at: @room_updated_at) if @room_updated_at
  end

  test "concurrent mentions cannot exceed the rate limit" do
    mention = mention_attachment_for(:bender)
    david_id = users(:david).id
    room_id = @room.id
    created_ids = @created_ids
    ids_mutex = Mutex.new
    ready = Queue.new
    go = Queue.new

    @membership_state = Membership.where(room_id: room_id).pluck(:id, :unread_at)
    @room_updated_at = @room.updated_at

    threads = 4.times.map do |t|
      Thread.new do
        ActiveRecord::Base.connection_pool.with_connection do
          ready << true
          go.pop

          7.times do |i|
            message = Message.create!(
              room_id: room_id, creator_id: david_id,
              body: "Race #{t}-#{i} #{mention}",
              client_message_id: "race-#{t}-#{i}-#{SecureRandom.hex(4)}"
            )
            ids_mutex.synchronize { created_ids << message.id }
          end
        end
      end
    end

    4.times { ready.pop }
    4.times { go << true }
    threads.each(&:join)

    rows = @agent.agent_events.where(message_id: created_ids)
    assert_equal 28, created_ids.size
    assert_equal 28, rows.count, "every mention writes exactly one row"
    assert_operator rows.deliverable.count, :<=, 20
    assert_equal 28 - rows.deliverable.count,
      rows.where(event_type: "delivery_suppressed_rate_limit").count
  end
end
