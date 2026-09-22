require "test_helper"

class RoomMentionsRenderTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "rendering a room page costs no extra queries per message with mentions" do
    create_mention_messages(2, offset: 0)

    # Warm per-process caches so the two measured renders share the same
    # constant cold-cache cost.
    get room_url(@room)
    assert_response :success

    small = count_queries { get room_url(@room) }
    assert_response :success

    create_mention_messages(4, offset: 2)
    large = count_queries { get room_url(@room) }
    assert_response :success

    assert_equal small, large,
      "room render should be O(1) in queries, got #{small} then #{large}"
  end

  private
    def create_mention_messages(count, offset:)
      count.times do |i|
        number = offset + i
        user = User.create!(name: "Mentioned #{number}")
        @room.memberships.find_or_create_by!(user:)
        @room.messages.create!(
          creator: users(:david),
          markdown_source: "hey @[Mentioned #{number}], see this",
          client_message_id: "mention-render-#{number}"
        )
      end
    end

    # Same shape as the count_queries in GithubPrCardsTest: every SQL
    # statement except schema loads and query-cache hits.
    def count_queries
      count = 0
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 unless payload[:name] == "SCHEMA" || payload[:cached]
      end

      ActiveRecord::Base.connection.clear_query_cache
      yield
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end
end
