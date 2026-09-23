require "test_helper"

class Calendar::PushChannelTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @user = users(:david)
    @webhook_before = ENV["GOOGLE_CALENDAR_WEBHOOK_URL"]
    ENV["GOOGLE_CALENDAR_WEBHOOK_URL"] = "https://app.test/google/calendar/notifications"
  end

  teardown do
    ENV["GOOGLE_CALENDAR_WEBHOOK_URL"] = @webhook_before
  end

  test "watching is disabled without a callback URL" do
    ENV.delete("GOOGLE_CALENDAR_WEBHOOK_URL")

    assert_not Calendar::PushChannel.watching_enabled?
    assert_nil Calendar::PushChannel.watch_for!(@user)
    assert_not_requested :post, %r{www\.googleapis\.com/calendar}
  end

  test "watching is disabled without a usable account" do
    assert_nil Calendar::PushChannel.watch_for!(@user)
    assert_not_requested :post, %r{www\.googleapis\.com/calendar}
  end

  test "watch_for opens a channel and stores only the token digest" do
    connect_google!(@user)
    watch = stub_request(:post, "#{GOOGLE_EVENTS_URL}/watch")
      .to_return(status: 200, body: {
        resourceId: "resource-1", expiration: (3.days.from_now.to_f * 1000).to_i.to_s
      }.to_json, headers: { "Content-Type" => "application/json" })

    channel = Calendar::PushChannel.watch_for!(@user)

    assert_requested watch, times: 1
    assert channel.persisted?
    assert_equal "resource-1", channel.resource_id
    assert channel.expires_at > 2.days.from_now
    assert_nil channel.attributes["token"]
    assert_equal 64, channel.token_digest.length
  end

  test "watch_for stops the previous channel first" do
    connect_google!(@user)
    old = Calendar::PushChannel.create!(user: @user, channel_id: "old-id",
      resource_id: "old-resource", token_digest: Calendar::PushChannel.digest("old-token"))
    stop = stub_request(:post, "https://www.googleapis.com/calendar/v3/channels/stop")
      .to_return(status: 200, body: {}.to_json)
    stub_request(:post, "#{GOOGLE_EVENTS_URL}/watch")
      .to_return(status: 200, body: { resourceId: "new-resource" }.to_json)

    channel = Calendar::PushChannel.watch_for!(@user)

    assert_requested stop, times: 1
    assert_equal old.id, channel.id
    assert_equal "new-resource", channel.resource_id
  end

  test "token matching is constant-time against the digest" do
    channel = Calendar::PushChannel.new(token_digest: Calendar::PushChannel.digest("secret-token"))

    assert channel.token_matches?("secret-token")
    assert_not channel.token_matches?("wrong-token")
    assert_not channel.token_matches?("")
    assert_not channel.token_matches?(nil)
  end

  test "notification claims dedupe by message number" do
    channel = Calendar::PushChannel.create!(user: @user, channel_id: "chan-1",
      token_digest: Calendar::PushChannel.digest("token"))

    assert channel.claim_notification!("7")
    assert_not channel.claim_notification!("7")
    assert_not channel.claim_notification!("3")
    assert channel.claim_notification!("8")
    assert_not channel.claim_notification!("bogus")
  end

  test "renew_expiring renews soon-expiring channels and leaves fresh ones" do
    connect_google!(@user)
    expiring = Calendar::PushChannel.create!(user: @user, channel_id: "old-id",
      resource_id: "old-resource", token_digest: Calendar::PushChannel.digest("old"),
      expires_at: 2.hours.from_now)
    stub_request(:post, "https://www.googleapis.com/calendar/v3/channels/stop")
      .to_return(status: 200, body: {}.to_json)
    watch = stub_request(:post, "#{GOOGLE_EVENTS_URL}/watch")
      .to_return(status: 200, body: { resourceId: "new-resource" }.to_json)

    Calendar::PushChannel.renew_expiring!

    assert_requested watch, times: 1
    assert_equal "new-resource", expiring.reload.resource_id
  end

  test "renew_expiring drops channels whose account went away" do
    channel = Calendar::PushChannel.create!(user: @user, channel_id: "old-id",
      resource_id: "old-resource", token_digest: Calendar::PushChannel.digest("old"),
      expires_at: 2.hours.from_now)

    Calendar::PushChannel.renew_expiring!

    assert_nil Calendar::PushChannel.find_by(id: channel.id)
  end

  test "renew_expiring never raises" do
    connect_google!(@user)
    Calendar::PushChannel.create!(user: @user, channel_id: "old-id",
      resource_id: "old-resource", token_digest: Calendar::PushChannel.digest("old"),
      expires_at: 2.hours.from_now)
    stub_request(:post, "https://www.googleapis.com/calendar/v3/channels/stop").to_timeout
    stub_request(:post, "#{GOOGLE_EVENTS_URL}/watch").to_timeout

    assert_nothing_raised { Calendar::PushChannel.renew_expiring! }
  end
end
