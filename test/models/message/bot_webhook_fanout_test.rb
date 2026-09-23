require "test_helper"

class Message::BotWebhookFanoutTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:watercooler)
    @legacy = User.create_bot!(name: "Legacy Note", webhook_url: "https://example.test/legacy-note")
    @room.memberships.grant_to(@legacy)
  end

  test "delivers a normal message mentioning a legacy bot" do
    message = @room.messages.create!(creator: users(:david),
      markdown_source: "Hey @[Legacy Note]", client_message_id: "fanout-normal")

    assert_enqueued_jobs 1, only: Bot::WebhookJob do
      Message::BotWebhookFanout.deliver_for(message)
    end
  end

  test "skips system notes mentioning a legacy bot" do
    note = @room.messages.create!(creator: users(:david),
      markdown_source: "Hey @[Legacy Note]", system_note: true, client_message_id: "fanout-note")

    assert_no_enqueued_jobs only: Bot::WebhookJob do
      Message::BotWebhookFanout.deliver_for(note)
    end
  end
end
