require "test_helper"

class WebhookAgentKeyTest < ActiveSupport::TestCase
  test "deliver without agent context for an agent-backed bot sends no agent key or bot key" do
    message = messages(:first)
    room_path = Rails.application.routes.url_helpers.room_path(message.room)

    captured = nil
    WebMock.stub_request(:post, webhooks(:bender).url)
      .with { |request| captured = request.body; true }
      .to_return(status: 200)

    webhooks(:bender).deliver(message)

    assert_requested :post, webhooks(:bender).url, body: hash_including(
      "room" => hash_including("path" => room_path)
    ), times: 1
    assert_not_includes captured, bot_key_for(users(:bender))
    assert_not_includes JSON.parse(captured).keys, "agent"
  end

  test "deliver for a legacy bot without an agent row sends the signed reply path as room.path" do
    legacy = User.create_bot!(name: "Legacy Path Bot", webhook_url: "https://example.test/legacy-path")
    key = legacy.plain_bot_key
    message = messages(:first)
    message.room.memberships.grant_to(legacy)

    captured = nil
    WebMock.stub_request(:post, legacy.webhook.url)
      .with { |request| captured = request.body; true }
      .to_return(status: 200)

    legacy.webhook.deliver(message)

    payload = JSON.parse(captured)
    reply_url = payload["reply_url"]
    assert_equal reply_url, payload.dig("room", "path"),
      "room.path carries the same signed reply path so old integrations keep posting back"
    assert_not_includes captured, key
    token = CGI.unescape(reply_url.split("/").fetch(-2))
    assert_equal legacy, User.authenticate_bot_reply_token(token, room_id: message.room.id)
    assert_not_includes payload.keys, "agent"
  end

  test "deliver for an agent-backed bot sends no reply url" do
    captured = nil
    WebMock.stub_request(:post, webhooks(:bender).url)
      .with { |request| captured = request.body; true }
      .to_return(status: 200)

    webhooks(:bender).deliver(messages(:first))

    assert_not_includes JSON.parse(captured).keys, "reply_url"
  end

  test "deliver with agent context adds the agent key alongside existing keys" do
    message = messages(:first)
    agent = agents(:bender_agent)

    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    webhooks(:bender).deliver(message, agent: agent, delivery_id: 123)

    assert_requested :post, webhooks(:bender).url, body: hash_including(
      "user" => hash_including("id" => message.creator.id),
      "room" => hash_including("id" => message.room.id),
      "message" => hash_including("id" => message.id),
      "agent" => {
        "id" => agent.id,
        "name" => "Bender Bot",
        "owner" => "David",
        "delivery_id" => 123
      }
    ), times: 1
  end

  test "agent key renders null owner for ownerless agents" do
    message = messages(:first)
    agent = agents(:bender_agent)
    agent.update_columns(owner_id: nil)

    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    webhooks(:bender).deliver(message, agent: agent, delivery_id: 7)

    assert_requested :post, webhooks(:bender).url, body: hash_including(
      "agent" => hash_including("owner" => nil, "delivery_id" => 7)
    ), times: 1
  end

  test "agent delivery in a PR thread carries the pull_request object" do
    room = rooms(:watercooler)
    parent = room.messages.create!(
      creator: users(:david),
      markdown_source: "review https://github.com/rails/rails/pull/12",
      client_message_id: "webhook-pr-parent"
    )
    pull_request = parent.github_pull_requests.first
    pull_request.update!(
      private: false, title: "Fix login", state: "open", base_branch: "main", head_branch: "shiny",
      review_decision: "approved", check_status: "passing",
      html_url: "https://github.com/rails/rails/pull/12"
    )
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "PR chat", parent_message: parent)
    Github::PullRequestThread.create!(pull_request: pull_request, room: room, channel_thread: thread)
    reply = thread.post_message!(
      creator: users(:david), attributes: { markdown_source: "hey", client_message_id: "webhook-pr-reply" }
    )

    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    webhooks(:bender).deliver(reply, agent: agents(:bender_agent), delivery_id: 9)

    assert_requested :post, webhooks(:bender).url, body: hash_including(
      "pull_request" => {
        "url" => "https://github.com/rails/rails/pull/12",
        "owner" => "rails",
        "repo" => "rails",
        "number" => 12,
        "title" => "Fix login",
        "state" => "open",
        "head_branch" => "shiny",
        "base_branch" => "main",
        "review_decision" => "approved",
        "checks_state" => "passing"
      }
    ), times: 1
  end

  test "agent delivery signs the raw body and timestamps the post" do
    message = messages(:first)
    agent = agents(:bender_agent)

    captured = nil
    WebMock.stub_request(:post, webhooks(:bender).url)
      .with { |request| captured = request; true }
      .to_return(status: 200)

    webhooks(:bender).deliver(message, agent: agent, delivery_id: 123)

    secret = agent.reload.webhook_signing_secret
    assert secret.present?, "the first delivery generates the agent secret"
    timestamp = timestamp_header(captured)
    expected = "sha256=#{OpenSSL::HMAC.hexdigest("SHA256", secret, "#{timestamp}.#{captured.body}")}"
    assert_equal expected, signature_header(captured)
    assert_match(/\A\d+\z/, timestamp)
  end

  test "approval, work, and completion deliveries sign with the agent secret" do
    agent = agents(:bender_agent)
    approval = AgentApproval.create!(agent: agent, action: "deploy", summary: "Ship it")
    event = agent.agent_events.create!(
      event_type: "github_action_completed", outcome: "delivered",
      metadata: { "approval_id" => approval.id, "action" => "github.comment", "status" => "completed" }
    )

    captured = []
    WebMock.stub_request(:post, webhooks(:bender).url)
      .with { |request| captured << request; true }
      .to_return(status: 200)

    Agent::Delivery.post_approval_webhook!(webhooks(:bender), approval, agent: agent, delivery_id: 7)
    Agent::Delivery.post_work_webhook!(webhooks(:bender), event, work: { "title" => "Signed work" }, agent: agent)
    Agent::Delivery.post_github_action_webhook!(webhooks(:bender), event, agent: agent)

    secret = agent.reload.webhook_signing_secret
    assert_equal 3, captured.size
    captured.each do |request|
      timestamp = timestamp_header(request)
      expected = "sha256=#{OpenSSL::HMAC.hexdigest("SHA256", secret, "#{timestamp}.#{request.body}")}"
      assert_equal expected, signature_header(request)
    end
  end

  test "agent delivery outside PR threads carries a null pull_request" do
    room = rooms(:watercooler)
    parent = room.messages.create!(
      creator: users(:david), markdown_source: "ordinary starter",
      client_message_id: "webhook-null-parent"
    )
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "Ordinary chat", parent_message: parent)
    reply = thread.post_message!(
      creator: users(:david), attributes: { markdown_source: "hey", client_message_id: "webhook-null-reply" }
    )
    room_message = room.messages.create!(
      creator: users(:david), markdown_source: "hey in the room",
      client_message_id: "webhook-null-room"
    )

    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    webhooks(:bender).deliver(reply, agent: agents(:bender_agent), delivery_id: 10)
    webhooks(:bender).deliver(room_message, agent: agents(:bender_agent), delivery_id: 11)

    assert_requested :post, webhooks(:bender).url, body: hash_including("pull_request" => nil), times: 2
  end

  private
    def signature_header(request)
      webhook_header(request, "x-smartfire-signature")
    end

    def timestamp_header(request)
      webhook_header(request, "x-smartfire-timestamp")
    end

    def webhook_header(request, name)
      value = request.headers.find { |key, _| key.to_s.downcase == name }&.last
      value.is_a?(Array) ? value.first : value
    end
end
