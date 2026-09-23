require "test_helper"

class Google::CalendarNotificationsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:david)
    @channel = Calendar::PushChannel.create!(user: @user, channel_id: "chan-1",
      resource_id: "resource-1", token_digest: Calendar::PushChannel.digest("channel-token"))
  end

  test "unknown channel answers 404 and enqueues nothing" do
    assert_no_enqueued_jobs only: Calendar::InboundSyncJob do
      post "/google/calendar/notifications", headers: notification_headers(channel_id: "nope")
    end

    assert_response :not_found
  end

  test "wrong token answers 403 and enqueues nothing" do
    assert_no_enqueued_jobs only: Calendar::InboundSyncJob do
      post "/google/calendar/notifications", headers: notification_headers(token: "wrong")
    end

    assert_response :forbidden
  end

  test "sync handshake is acknowledged without work" do
    assert_no_enqueued_jobs only: Calendar::InboundSyncJob do
      post "/google/calendar/notifications", headers: notification_headers(state: "sync")
    end

    assert_response :ok
  end

  test "a change notification enqueues one inbound sync" do
    assert_enqueued_with(job: Calendar::InboundSyncJob, args: [ @user.id ]) do
      post "/google/calendar/notifications", headers: notification_headers(number: "12")
    end

    assert_response :ok
    assert_equal 12, @channel.reload.last_message_number
  end

  test "a redelivered notification is acknowledged without a second sync" do
    post "/google/calendar/notifications", headers: notification_headers(number: "12")
    assert_response :ok

    assert_no_enqueued_jobs only: Calendar::InboundSyncJob do
      post "/google/calendar/notifications", headers: notification_headers(number: "12")
    end

    assert_response :ok
  end

  test "not_exists drops the channel and enqueues a re-watch" do
    assert_enqueued_with(job: Calendar::WatchChannelJob, args: [ @user.id ]) do
      post "/google/calendar/notifications", headers: notification_headers(state: "not_exists")
    end

    assert_response :ok
    assert_nil Calendar::PushChannel.find_by(id: @channel.id)
  end

  private
    def notification_headers(channel_id: "chan-1", token: "channel-token", state: "exists", number: "1")
      {
        "X-Goog-Channel-Id" => channel_id,
        "X-Goog-Channel-Token" => token,
        "X-Goog-Resource-Id" => "resource-1",
        "X-Goog-Resource-State" => state,
        "X-Goog-Message-Number" => number
      }
    end
end
