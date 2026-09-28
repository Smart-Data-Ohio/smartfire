require "test_helper"

class Slack::ClientTest < ActiveSupport::TestCase
  include SlackImportTestHelper

  setup do
    @client = Slack::Client.new(token: SLACK_TEST_TOKEN)
    @auth = { "Authorization" => slack_auth_header }
  end

  test "auth.test sends a bearer user token" do
    stub = stub_request(:get, "#{SLACK_API}/auth.test")
      .with(headers: @auth)
      .to_return(status: 200, body: JSON.generate({ ok: true, user_id: "UADMIN", team_id: "T123" }))

    payload = @client.auth_test

    assert_requested stub
    assert_equal "UADMIN", payload["user_id"]
  end

  test "team.info" do
    stub_request(:get, "#{SLACK_API}/team.info").with(headers: @auth)
      .to_return(status: 200, body: JSON.generate({ ok: true, team: { id: "T123", name: "Smart Data" } }))

    assert_equal "Smart Data", @client.team_info["team"]["name"]
  end

  test "users.list pages with cursor and limit" do
    stub = stub_request(:get, "#{SLACK_API}/users.list")
      .with(query: { "limit" => "200", "cursor" => "abc" }, headers: @auth)
      .to_return(status: 200, body: slack_fixture("users.json"))

    page = @client.users_list(cursor: "abc")

    assert_requested stub
    assert_equal 9, page["members"].size
  end

  test "conversations.list sends types and keeps archived channels" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.list")
      .with(query: { "types" => "public_channel,private_channel", "exclude_archived" => "false", "limit" => "200" },
        headers: @auth)
      .to_return(status: 200, body: slack_fixture("conversations_workspace.json"))

    page = @client.conversations_list(types: "public_channel,private_channel")

    assert_requested stub
    assert_equal 3, page["channels"].size
  end

  test "conversations.members sends the channel" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.members")
      .with(query: { "channel" => "CCHAN", "limit" => "1000" }, headers: @auth)
      .to_return(status: 200, body: slack_fixture("members_CCHAN.json"))

    page = @client.conversations_members(channel: "CCHAN")

    assert_requested stub
    assert_equal %w[ UADMIN U001 U002 U003 ], page["members"]
  end

  test "conversations.history sends channel, bounds, cursor and limit 200" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.history")
      .with(query: { "channel" => "CCHAN", "oldest" => "1700000000.000000",
        "latest" => "1700000060.000000", "cursor" => "cchan-page-2", "limit" => "200" },
        headers: @auth)
      .to_return(status: 200, body: slack_fixture("history_CCHAN_p2.json"))

    page = @client.conversations_history(channel: "CCHAN", oldest: "1700000000.000000",
      latest: "1700000060.000000", cursor: "cchan-page-2")

    assert_requested stub
    assert_equal 1, page["messages"].size
  end

  test "conversations.replies sends channel and parent ts" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.replies")
      .with(query: { "channel" => "CCHAN", "ts" => "1700000002.000002", "limit" => "200" },
        headers: @auth)
      .to_return(status: 200, body: slack_fixture("replies_CCHAN_parent.json"))

    page = @client.conversations_replies(channel: "CCHAN", ts: "1700000002.000002")

    assert_requested stub
    assert_equal 3, page["messages"].size
  end

  test "HTTP 429 raises a rate-limit error carrying Retry-After" do
    stub_request(:get, "#{SLACK_API}/conversations.history").with(query: hash_including({ "channel" => "CCHAN" }), headers: @auth)
      .to_return(status: 429, headers: { "Retry-After" => "17" })

    error = assert_raises(Slack::Client::RateLimited) do
      @client.conversations_history(channel: "CCHAN")
    end

    assert_equal 17, error.retry_after
  end

  test "HTTP 429 without Retry-After defaults to 60 seconds" do
    stub_request(:get, "#{SLACK_API}/users.list").with(query: hash_including({ "limit" => "200" }), headers: @auth)
      .to_return(status: 429)

    error = assert_raises(Slack::Client::RateLimited) { @client.users_list }

    assert_equal 60, error.retry_after
  end

  test "5xx responses retry with backoff then raise" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.history").with(query: hash_including({ "channel" => "CCHAN" }), headers: @auth)
      .to_return(status: 500, body: "boom")

    assert_raises(Slack::Client::RequestError) { @client.conversations_history(channel: "CCHAN") }

    assert_requested stub, times: Slack::Client::MAX_ATTEMPTS
  end

  test "network errors retry then raise" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.history").with(query: hash_including({ "channel" => "CCHAN" }), headers: @auth)
      .to_raise(Errno::ECONNRESET)

    error = assert_raises(Slack::Client::RequestError) do
      @client.conversations_history(channel: "CCHAN")
    end
    assert_includes error.message, "ECONNRESET"

    assert_requested stub, times: Slack::Client::MAX_ATTEMPTS
  end

  test "a 5xx that recovers returns the payload" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.history").with(query: hash_including({ "channel" => "CARCH" }), headers: @auth)
      .to_return(status: 500, body: "boom")
      .to_return(status: 200, body: slack_fixture("history_CARCH.json"))

    page = @client.conversations_history(channel: "CARCH")

    assert_requested stub, times: 2
    assert_equal 1, page["messages"].size
  end

  test "ok:false auth errors raise AuthError" do
    %w[ invalid_auth token_revoked account_inactive not_authed ].each do |code|
      stub_request(:get, "#{SLACK_API}/users.list").with(query: hash_including({ "limit" => "200" }), headers: @auth)
        .to_return(status: 200, body: JSON.generate({ ok: false, error: code }))

      error = assert_raises(Slack::Client::AuthError, "expected AuthError for #{code}") do
        @client.users_list
      end
      assert_includes error.message, code
    end
  end

  test "ok:false missing_scope raises ScopeError with the needed scope" do
    stub_request(:get, "#{SLACK_API}/conversations.history").with(query: hash_including({ "channel" => "CCHAN" }), headers: @auth)
      .to_return(status: 200, body: JSON.generate({ ok: false, error: "missing_scope",
        needed: "channels:history", provided: "channels:read" }))

    error = assert_raises(Slack::Client::ScopeError) do
      @client.conversations_history(channel: "CCHAN")
    end

    assert_includes error.message, "channels:history"
    assert_equal "channels:history", error.needed
  end

  test "other ok:false errors raise RequestError without retrying" do
    stub = stub_request(:get, "#{SLACK_API}/conversations.history").with(query: hash_including({ "channel" => "CNOPE" }), headers: @auth)
      .to_return(status: 200, body: JSON.generate({ ok: false, error: "channel_not_found" }))

    error = assert_raises(Slack::Client::RequestError) do
      @client.conversations_history(channel: "CNOPE")
    end

    assert_requested stub, times: 1
    assert_includes error.message, "channel_not_found"
  end

  test "on_request fires once per attempt for api call counts" do
    calls = []
    client = Slack::Client.new(token: SLACK_TEST_TOKEN, on_request: ->(method) { calls << method })
    stub_request(:get, "#{SLACK_API}/conversations.history").with(query: hash_including({ "channel" => "CARCH" }), headers: @auth)
      .to_return(status: 500, body: "boom")
      .to_return(status: 200, body: slack_fixture("history_CARCH.json"))

    client.conversations_history(channel: "CARCH")

    assert_equal %w[ conversations.history conversations.history ], calls
  end

  test "pacing sleeps between rapid Tier 2 calls when enabled" do
    client = Slack::Client.new(token: SLACK_TEST_TOKEN, pacing: true)
    stub_request(:get, "#{SLACK_API}/users.list").with(query: hash_including({ "limit" => "200" }), headers: @auth)
      .to_return(status: 200, body: JSON.generate({ ok: true, members: [] }))
    slept = 0.0
    client.define_singleton_method(:sleep) { |seconds| slept += seconds }

    client.users_list
    client.users_list

    assert slept > 0, "expected pacing sleep between Tier 2 calls, slept #{slept}"
  end
end
