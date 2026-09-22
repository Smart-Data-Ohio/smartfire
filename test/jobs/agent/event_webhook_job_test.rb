require "test_helper"

class Agent::EventWebhookJobTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
    @webhook_url = webhooks(:bender).url
  end

  test "a successful post marks the row delivered" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 200)

    event = deliverable_event
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "delivered", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_nil event.reload.webhook_last_error
    assert_requested :post, @webhook_url, times: 1
  end

  test "a transport failure records the attempt and retries with backoff" do
    WebMock.stub_request(:post, @webhook_url).to_raise(Errno::ECONNREFUSED)

    event = deliverable_event
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "pending", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_match "Connection refused", event.reload.webhook_last_error.to_s
    assert_enqueued_jobs 1, only: Agent::EventWebhookJob

    WebMock.stub_request(:post, @webhook_url).to_return(status: 200)
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "delivered", event.reload.webhook_status
    assert_equal 2, event.reload.webhook_attempts
    assert_nil event.reload.webhook_last_error
    assert_requested :post, @webhook_url, times: 2
  end

  test "a timeout records the attempt in the ledger and retries" do
    WebMock.stub_request(:post, @webhook_url).to_timeout

    event = deliverable_event

    assert_no_difference -> { Message.count } do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "pending", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_match "OpenTimeout", event.reload.webhook_last_error.to_s
    assert_enqueued_jobs 1, only: Agent::EventWebhookJob
  end

  test "the fifth failure marks the row failed without re-enqueueing" do
    WebMock.stub_request(:post, @webhook_url).to_raise(Errno::ECONNREFUSED)

    event = deliverable_event
    event.update!(webhook_attempts: 4)

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "failed", event.reload.webhook_status
    assert_equal 5, event.reload.webhook_attempts
    assert_match "Connection refused", event.reload.webhook_last_error.to_s
  end

  test "a 201 response counts as delivered" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 201)

    event = deliverable_event
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "delivered", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_nil event.reload.webhook_last_error
  end

  test "a 500 response retries with backoff and fails after five attempts" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 500, body: "boom")

    event = deliverable_event
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "pending", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_match "500", event.reload.webhook_last_error.to_s
    assert_enqueued_jobs 1, only: Agent::EventWebhookJob

    event.update!(webhook_attempts: 4)
    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "failed", event.reload.webhook_status
    assert_equal 5, event.reload.webhook_attempts
    assert_match "500", event.reload.webhook_last_error.to_s
  end

  test "a 429 response waits out Retry-After before retrying" do
    WebMock.stub_request(:post, @webhook_url)
      .to_return(status: 429, headers: { "Retry-After" => "45" })

    event = deliverable_event
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "pending", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_match "429", event.reload.webhook_last_error.to_s

    job = enqueued_jobs.select { |enqueued| enqueued[:job] == Agent::EventWebhookJob }.sole
    assert_in_delta 45, job[:at] - Time.current.to_f, 5
  end

  test "a 429 response without Retry-After retries with the default backoff" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 429)

    event = deliverable_event
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "pending", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_enqueued_jobs 1, only: Agent::EventWebhookJob
  end

  test "a 408 response retries instead of failing fast" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 408)

    event = deliverable_event
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "pending", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts
    assert_enqueued_jobs 1, only: Agent::EventWebhookJob
  end

  test "a 404 response fails fast without retrying" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 404, body: "gone")

    event = deliverable_event

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "failed", event.reload.webhook_status
    assert_equal 0, event.reload.webhook_attempts
    assert_match "404", event.reload.webhook_last_error.to_s
  end

  test "an error response with an attachment body creates no sync reply" do
    WebMock.stub_request(:post, @webhook_url)
      .to_return(status: 500, body: file_fixture("moon.jpg").read, headers: { "Content-Type" => "image/jpeg" })

    event = deliverable_event

    assert_no_difference -> { Message.count } do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "pending", event.reload.webhook_status
  end

  test "a guard refusal fails fast without retrying" do
    bot = User.create_bot!(name: "Loopback Webhook Bot", webhook_url: "http://127.0.0.1:9999/hook")
    agent = bot.create_agent!(kind: :workspace, owner: users(:david))
    @room.memberships.grant_to(bot)
    message = @room.messages.create!(
      creator: users(:david), body: "Hey", client_message_id: "webhook-guard-1"
    )
    event = agent.agent_events.create!(
      event_type: "mention", room: @room, message: message,
      outcome: "delivered", webhook_status: "pending"
    )

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "failed", event.reload.webhook_status
    assert_equal 0, event.reload.webhook_attempts
    assert_match "Violation", event.reload.webhook_last_error.to_s
    assert_not_requested :post, bot.webhook.url
  end

  test "an unresolvable host fails fast without retrying" do
    stub_dns_failure

    event = deliverable_event

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "failed", event.reload.webhook_status
    assert_match "Unresolvable", event.reload.webhook_last_error.to_s
    assert_not_requested :post, @webhook_url
  end

  test "a message deleted before the post fails fast without retrying" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 200)

    event = deliverable_event
    event.message.destroy!

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::EventWebhookJob.perform_now(event.id)
    end

    assert_equal "failed", event.reload.webhook_status
    assert_match "no longer available", event.reload.webhook_last_error.to_s
    assert_not_requested :post, @webhook_url
  end

  test "a removed webhook configuration clears the owed post" do
    event = deliverable_event
    webhooks(:bender).destroy!

    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "none", event.reload.webhook_status
  end

  test "an already delivered row posts nothing" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 200)

    event = deliverable_event
    event.update!(webhook_status: "delivered", webhook_attempts: 1)

    Agent::EventWebhookJob.perform_now(event.id)

    assert_not_requested :post, @webhook_url
  end

  test "acking a pending row does not cancel its webhook" do
    WebMock.stub_request(:post, @webhook_url).to_return(status: 200)

    @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
      client_message_id: "webhook-ack-1"
    )
    event = @agent.agent_events.deliverable.last
    event.acknowledged!

    assert_enqueued_jobs 1, only: Agent::EventWebhookJob do
      Agent::DeliveryJob.perform_now(event.id)
    end
    perform_enqueued_jobs only: Agent::EventWebhookJob

    assert_equal "acknowledged", event.reload.outcome
    assert_equal "delivered", event.reload.webhook_status
    assert_requested :post, @webhook_url, times: 1
  end

  private
    def deliverable_event
      @room.messages.create!(
        creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
        client_message_id: "webhook-job-#{SecureRandom.hex(4)}"
      )
      event = @agent.agent_events.deliverable.last
      event.update!(outcome: "delivered", webhook_status: "pending")
      event
    end
end
