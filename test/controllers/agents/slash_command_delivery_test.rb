require "test_helper"

class Agents::SlashCommandDeliveryTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:watercooler)
    @agent = agents(:bender_agent)
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
    AgentSlashCommand.create!(agent: @agent, room: @room, name: "deploy")
  end

  test "a thread invocation posts the thread id to the webhook" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Side chat")

    SlashCommands::Dispatcher.dispatch(user: users(:david), room: @room, thread: thread, text: "/deploy staging")
    assert_not_requested :post, webhooks(:bender).url

    perform_enqueued_jobs only: Agent::EventWebhookJob

    assert_requested :post, webhooks(:bender).url, body: hash_including(
      "event_type" => "slash_command",
      "thread_id" => thread.id,
      "command" => hash_including("name" => "deploy", "arguments" => "staging")
    ), times: 1
  end

  test "a channel invocation posts no thread id to the webhook" do
    SlashCommands::Dispatcher.dispatch(user: users(:david), room: @room, text: "/deploy staging")

    perform_enqueued_jobs only: Agent::EventWebhookJob

    assert_requested :post, webhooks(:bender).url, body: hash_including(
      "event_type" => "slash_command",
      "command" => hash_including("name" => "deploy")
    ), times: 1
    assert_requested :post, webhooks(:bender).url, body: hash_excluding("thread_id"), times: 1
  end
end
