require "test_helper"

class WebhookTest < ActiveSupport::TestCase
  test "payload" do
    message = messages(:first)
    message_path = Rails.application.routes.url_helpers.room_at_message_path(message.room, message)
    room_path = Rails.application.routes.url_helpers.room_path(message.room)

    captured = nil
    WebMock.stub_request(:post, webhooks(:bender).url)
      .with { |request| captured = request.body; true }
      .to_return(status: 200)

    response = webhooks(:bender).deliver(messages(:first))
    assert_equal 200, response.code.to_i

    payload = JSON.parse(captured)
    assert_equal message.creator.id, payload.dig("user", "id")
    assert_equal message.creator.name, payload.dig("user", "name")
    assert_equal room_path, payload.dig("room", "path")
    assert_equal message.id, payload.dig("message", "id")
    assert_equal "First post!", payload.dig("message", "body", "html")
    assert_equal "First post!", payload.dig("message", "body", "plain")
    assert_equal message_path, payload.dig("message", "path")
    assert_not_includes captured, users(:bender).bot_key
  end

  test "delivery" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200, body: "", headers: {})
    response = webhooks(:bender).deliver(messages(:first))
    assert_equal 200, response.code.to_i
  end

  test "delivery with OK text reply" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200, body: "Hello back!", headers: { "Content-Type" => "text/plain" })
    response = webhooks(:bender).deliver(messages(:first))

    reply_message = Message.last
    assert_equal "Hello back!", reply_message.body.to_plain_text
  end

  test "sync text reply lands in the triggering message's thread" do
    room = rooms(:watercooler)
    room.memberships.grant_to(users(:bender)) unless room.memberships.exists?(user: users(:bender))
    parent = room.messages.create!(
      creator: users(:david), markdown_source: "Thread starter", client_message_id: "sync-thread-parent"
    )
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "Sync chat", parent_message: parent)
    trigger = thread.post_message!(
      creator: users(:david), attributes: { markdown_source: "hey bot", client_message_id: "sync-thread-trigger" }
    )
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 200, body: "Thread answer", headers: { "Content-Type" => "text/plain" })

    webhooks(:bender).deliver(trigger, agent: agents(:bender_agent), delivery_id: 1)

    reply = Message.last
    assert_equal "Thread answer", reply.body.to_plain_text
    assert_equal thread.id, reply.thread_id
    assert_equal trigger.id, reply.reply_to_message_id
  end

  test "legacy sync reply lands in the triggering message's thread" do
    room = rooms(:watercooler)
    legacy = User.create_bot!(name: "Legacy Sync", webhook_url: "https://example.test/legacy-sync")
    room.memberships.grant_to(legacy)
    parent = room.messages.create!(
      creator: users(:david), markdown_source: "Legacy starter", client_message_id: "sync-legacy-parent"
    )
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "Legacy chat", parent_message: parent)
    trigger = thread.post_message!(
      creator: users(:david), attributes: { markdown_source: "hey legacy", client_message_id: "sync-legacy-trigger" }
    )
    WebMock.stub_request(:post, legacy.webhook.url)
      .to_return(status: 200, body: "Legacy answer", headers: { "Content-Type" => "text/plain" })

    legacy.webhook.deliver(trigger)

    reply = Message.last
    assert_equal thread.id, reply.thread_id
    assert_equal trigger.id, reply.reply_to_message_id
  end

  test "sync reply to a board post lands in the post" do
    board = Rooms::Board.create_for({ name: "Sync Board", creator: users(:david) }, users: [ users(:david) ])
    board.memberships.grant_to(users(:bender))
    post = ChannelThread.create_board_post!(
      room: board, creator: users(:david), name: "Sync post",
      work_status: "in_progress", first_message: "Board hello"
    )
    trigger = post.messages.first
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 200, body: "Post answer", headers: { "Content-Type" => "text/plain" })

    webhooks(:bender).deliver(trigger, agent: agents(:bender_agent), delivery_id: 2)

    reply = Message.last
    assert_equal "Post answer", reply.body.to_plain_text
    assert_equal post.id, reply.thread_id
    assert_equal trigger.id, reply.reply_to_message_id
  end

  test "sync reply to a root message references the trigger" do
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 200, body: "Root answer", headers: { "Content-Type" => "text/plain" })

    webhooks(:bender).deliver(messages(:first), agent: agents(:bender_agent), delivery_id: 3)

    reply = Message.last
    assert_nil reply.thread_id
    assert_equal messages(:first).id, reply.reply_to_message_id
  end

  test "sync attachment reply lands in the triggering message's thread" do
    room = rooms(:watercooler)
    room.memberships.grant_to(users(:bender)) unless room.memberships.exists?(user: users(:bender))
    parent = room.messages.create!(
      creator: users(:david), markdown_source: "Attach starter", client_message_id: "sync-attach-parent"
    )
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "Attach chat", parent_message: parent)
    trigger = thread.post_message!(
      creator: users(:david), attributes: { markdown_source: "hey bot", client_message_id: "sync-attach-trigger" }
    )
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 200, body: file_fixture("moon.jpg"), headers: { "Content-Type" => "image/jpeg" })

    webhooks(:bender).deliver(trigger, agent: agents(:bender_agent), delivery_id: 4)

    reply = Message.last
    assert reply.attachment.present?
    assert_equal thread.id, reply.thread_id
    assert_equal trigger.id, reply.reply_to_message_id
  end

  test "agent delivery that times out raises instead of posting" do
    Webhook.any_instance.stubs(:post).raises(Net::OpenTimeout)

    assert_no_difference -> { Message.count } do
      assert_raises Net::OpenTimeout do
        webhooks(:bender).deliver(messages(:first), agent: agents(:bender_agent), delivery_id: 5)
      end
    end
  end

  test "agent sync reply into a locked thread is logged, not raised" do
    room = rooms(:watercooler)
    room.memberships.grant_to(users(:bender)) unless room.memberships.exists?(user: users(:bender))
    parent = room.messages.create!(
      creator: users(:david), markdown_source: "Locked starter", client_message_id: "sync-locked-parent"
    )
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "Locked chat", parent_message: parent)
    trigger = thread.post_message!(
      creator: users(:david), attributes: { markdown_source: "hey bot", client_message_id: "sync-locked-trigger" }
    )
    thread.lock_conversation!
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 200, body: "Too late", headers: { "Content-Type" => "text/plain" })

    assert_no_difference -> { Message.count } do
      webhooks(:bender).deliver(trigger, agent: agents(:bender_agent), delivery_id: 6)
    end
  end

  test "legacy delivery without a secret sends no signature" do
    captured = nil
    WebMock.stub_request(:post, webhooks(:bender).url)
      .with { |request| captured = request; true }
      .to_return(status: 200)

    webhooks(:bender).deliver(messages(:first))

    assert_nil webhook_header(captured, "x-smartfire-signature")
    assert_match(/\A\d+\z/, webhook_header(captured, "x-smartfire-timestamp"))
  end

  test "legacy delivery with a secret signs the body" do
    webhooks(:bender).reset_signing_secret!

    captured = nil
    WebMock.stub_request(:post, webhooks(:bender).url)
      .with { |request| captured = request; true }
      .to_return(status: 200)

    webhooks(:bender).deliver(messages(:first))

    secret = webhooks(:bender).reload.signing_secret
    expected = "sha256=#{OpenSSL::HMAC.hexdigest("SHA256", secret, captured.body)}"
    assert_equal expected, webhook_header(captured, "x-smartfire-signature")
  end

  test "legacy sync reply into a locked thread still raises" do
    room = rooms(:watercooler)
    legacy = User.create_bot!(name: "Legacy Locked", webhook_url: "https://example.test/legacy-locked")
    room.memberships.grant_to(legacy)
    parent = room.messages.create!(
      creator: users(:david), markdown_source: "Locked starter", client_message_id: "sync-locked-legacy-parent"
    )
    thread = ChannelThread.create!(room: room, creator: users(:david), name: "Locked chat", parent_message: parent)
    trigger = thread.post_message!(
      creator: users(:david), attributes: { markdown_source: "hey legacy", client_message_id: "sync-locked-legacy-trigger" }
    )
    thread.lock_conversation!
    WebMock.stub_request(:post, legacy.webhook.url)
      .to_return(status: 200, body: "Too late", headers: { "Content-Type" => "text/plain" })

    assert_raises ChannelThread::LockedError do
      legacy.webhook.deliver(trigger)
    end
  end

  test "delivery with OK attachment reply" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200, body: file_fixture("moon.jpg"), headers: { "Content-Type" => "image/jpeg" })
    response = webhooks(:bender).deliver(messages(:first))

    reply_message = Message.last
    assert reply_message.attachment.present?
  end

  test "delivery with error reply" do
    assert_no_difference -> { Message.count } do
      WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 500, body: "Internal Error!", headers: {})
      response = webhooks(:bender).deliver(messages(:first))
    end
  end

  test "delivery that times out" do
    Webhook.any_instance.stubs(:post).raises(Net::OpenTimeout)
    response = webhooks(:bender).deliver(messages(:first))

    reply_message = Message.last
    assert_equal "Failed to respond within 7 seconds", reply_message.body.to_plain_text
  end

  test "delivery to a loopback URL is refused without posting" do
    bot = User.create_bot!(name: "Loopback Bot", webhook_url: "http://127.0.0.1:9999/hook")

    assert_raises RestrictedHTTP::Violation do
      bot.webhook.deliver(messages(:first))
    end

    assert_not_requested :post, bot.webhook.url
  end

  test "delivery to a hostname resolving to a private address is refused" do
    stub_dns_resolution("10.0.0.5")

    assert_raises RestrictedHTTP::Violation do
      webhooks(:bender).deliver(messages(:first))
    end

    assert_not_requested :post, webhooks(:bender).url
  end

  test "agent delivery to a refused URL raises instead of replying" do
    bot = User.create_bot!(name: "Private Agent Bot", webhook_url: "http://192.168.1.10/hook")
    agent = bot.create_agent!(kind: :workspace, owner: users(:david))

    assert_no_difference -> { Message.count } do
      assert_raises RestrictedHTTP::Violation do
        bot.webhook.deliver(messages(:first), agent: agent, delivery_id: 1)
      end
    end
  end

  test "agent approval webhook to a refused URL raises" do
    bot = User.create_bot!(name: "Private Approval Bot", webhook_url: "http://10.1.2.3/hook")
    agent = bot.create_agent!(kind: :workspace, owner: users(:david))
    approval = AgentApproval.create!(agent: agent, action: "deploy", summary: "Ship it")

    assert_raises RestrictedHTTP::Violation do
      Agent::Delivery.post_approval_webhook!(bot.webhook, approval, agent: agent, delivery_id: 1)
    end

    assert_not_requested :post, bot.webhook.url
  end

  test "delivery to an unresolvable hostname raises without posting" do
    stub_dns_failure

    assert_raises Surfguard::Unresolvable do
      webhooks(:bender).deliver(messages(:first))
    end

    assert_not_requested :post, webhooks(:bender).url
  end

  private
    def webhook_header(request, name)
      value = request.headers.find { |key, _| key.to_s.downcase == name }&.last
      value.is_a?(Array) ? value.first : value
    end
end
