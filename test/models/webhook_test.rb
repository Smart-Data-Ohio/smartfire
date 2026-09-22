require "test_helper"

class WebhookTest < ActiveSupport::TestCase
  test "payload" do
    message = messages(:first)
    message_path = Rails.application.routes.url_helpers.room_at_message_path(message.room, message)
    bot_messages_path = Rails.application.routes.url_helpers.room_bot_messages_path(message.room, users(:bender).bot_key)

    WebMock.stub_request(:post, webhooks(:bender).url).
      with(body: hash_including(
        user: { id: message.creator.id, name: message.creator.name },
        room: { id: message.room.id, name: message.room.name, path: bot_messages_path },
        message: { id: message.id, body: { html: "First post!", plain: "First post!" }, path: message_path },
      ))

    response = webhooks(:bender).deliver(messages(:first))
    assert_equal 200, response.code.to_i
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
end
