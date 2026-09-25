require "test_helper"

# The "N replies" thread indicator under room messages, as rooms#show and
# the paginated messages#index render it.
class ThreadIndicatorRenderingTest < ActionDispatch::IntegrationTest
  setup do
    host! "smartfire.test"
    sign_in :david
    @room = rooms(:designers)
  end

  test "show renders every message on the page" do
    get room_url(@room)

    assert_response :success
    @room.root_messages.each do |message|
      assert_select "#" + dom_id(message) + " .message__body", 1
    end
  end

  test "show renders each thread root's reply count and hides the rest" do
    replied = start_thread(messages(:first), replies: 2)
    start_thread(messages(:second), replies: 0)

    get room_url(@room)

    assert_indicator messages(:first), "2 replies"
    assert_select "##{dom_id(messages(:second), :thread_indicator)}[hidden]", 1
    assert_select "##{dom_id(messages(:third), :thread_indicator)}[hidden]", 1
    assert_equal 2, replied.reload.messages_count
  end

  test "the paginated messages index renders reply counts" do
    start_thread(messages(:third), replies: 3)

    get room_messages_url(@room)

    assert_response :success
    assert_indicator messages(:third), "3 replies"
  end

  test "show costs no extra queries per threaded message" do
    start_thread(messages(:first), replies: 1)
    get room_url(@room) # warm per-session lookups so both counts compare like for like
    one_thread = count_queries { get room_url(@room) }

    start_thread(messages(:second), replies: 2)
    start_thread(messages(:third), replies: 3)
    three_threads = count_queries { get room_url(@room) }

    assert_response :success
    assert_indicator messages(:third), "3 replies"
    assert_equal one_thread, three_threads
  end

  test "the paginated index costs no extra queries per threaded message" do
    start_thread(messages(:first), replies: 1)
    get room_messages_url(@room) # warm per-session lookups so both counts compare like for like
    one_thread = count_queries { get room_messages_url(@room) }

    start_thread(messages(:second), replies: 2)
    start_thread(messages(:third), replies: 3)
    three_threads = count_queries { get room_messages_url(@room) }

    assert_response :success
    assert_indicator messages(:second), "2 replies"
    assert_equal one_thread, three_threads
  end

  # Guards the channel_thread preload: without it every message on the page
  # loads its thread on its own, so more messages or more thread roots
  # would cost more queries.
  test "show costs no extra queries for more messages or thread roots" do
    get room_url(@room)
    baseline = count_queries { get room_url(@room) }

    add_plain_messages(2)
    more_messages = count_queries { get room_url(@room) }

    start_thread(messages(:first), replies: 1)
    more_roots = count_queries { get room_url(@room) }

    assert_equal baseline, more_messages
    assert_equal baseline, more_roots
  end

  test "the paginated index costs no extra queries for more messages or thread roots" do
    get room_messages_url(@room)
    baseline = count_queries { get room_messages_url(@room) }

    add_plain_messages(2)
    more_messages = count_queries { get room_messages_url(@room) }

    start_thread(messages(:first), replies: 1)
    more_roots = count_queries { get room_messages_url(@room) }

    assert_equal baseline, more_messages
    assert_equal baseline, more_roots
  end

  private
    # Posted as the signed-in member, so the room's unread state (and the
    # divider's lookups) stays as it was.
    def add_plain_messages(count)
      count.times do |index|
        @room.root_messages.create!(creator: users(:david), markdown_source: "Plain #{index}", client_message_id: "plain-#{index}")
      end
    end

    def start_thread(parent, replies:)
      thread = ChannelThread.create!(room: @room, creator: users(:david), parent_message: parent, name: "Thread on #{parent.id}")
      replies.times { |index| thread.post_message!(creator: users(:david), attributes: { markdown_source: "Reply #{index}" }) }
      thread
    end

    def assert_indicator(message, text)
      assert_select "##{dom_id(message, :thread_indicator)}:not([hidden])", 1 do
        assert_select "span", text:
      end
    end

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
