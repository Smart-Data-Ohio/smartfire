require "test_helper"

class Agent::DeliveryRecoveryTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:watercooler)
    @bot = users(:bender)
    @agent = agents(:bender_agent)
  end

  test "a stranded pending row gets recovered" do
    event = stranded_event(age: 3.minutes, attempts: 0)

    assert_enqueued_jobs 1, only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end

    job = enqueued_jobs.select { |enqueued| enqueued[:job] == Agent::EventWebhookJob }.sole
    assert_equal event.id, job[:args].first
  end

  test "a stranded row with attempts remaining but some spent gets recovered" do
    stranded_event(age: 10.minutes, attempts: 3)

    assert_enqueued_jobs 1, only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end
  end

  test "a fresh pending row is left alone" do
    stranded_event(age: 1.minute, attempts: 0)

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end
  end

  test "an exhausted pending row past its grace is marked failed" do
    old_null = stranded_event(age: 10.minutes, attempts: Agent::EventWebhookJob::MAX_ATTEMPTS)
    old_scheduled = stranded_event(age: 10.minutes, attempts: Agent::EventWebhookJob::MAX_ATTEMPTS)
    old_scheduled.update!(webhook_next_attempt_at: 8.minutes.ago)

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end

    assert_equal "failed", old_null.reload.webhook_status
    assert_equal "failed", old_scheduled.reload.webhook_status
  end

  test "a recently exhausted pending row is left alone" do
    event = stranded_event(age: 3.minutes, attempts: Agent::EventWebhookJob::MAX_ATTEMPTS)

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end

    assert_equal "pending", event.reload.webhook_status
  end

  test "settled webhook rows are left alone" do
    stranded_event(age: 10.minutes, attempts: 1, webhook_status: "delivered")
    stranded_event(age: 10.minutes, attempts: 5, webhook_status: "failed")

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end
  end

  test "a 429 with Retry-After 600 is not re-posted by the sweeper before 600 seconds" do
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 429, headers: { "Retry-After" => "600" })

    event = stranded_event(age: 0.seconds, attempts: 0)
    event.update!(created_at: Time.current)
    Agent::EventWebhookJob.perform_now(event.id)
    assert_equal "pending", event.reload.webhook_status
    assert_equal 1, event.reload.webhook_attempts

    travel 3.minutes do
      assert_no_enqueued_jobs only: Agent::EventWebhookJob do
        Agent::Delivery.recover_stranded_webhooks!
      end
    end

    travel 599.seconds do
      assert_no_enqueued_jobs only: Agent::EventWebhookJob do
        Agent::Delivery.recover_stranded_webhooks!(now: Time.current)
      end
    end
  end

  test "the sweeper does not overwrite a Retry-After stored after its read" do
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 429, headers: { "Retry-After" => "600" })
    event = stranded_event(age: 3.minutes, attempts: 0)

    # The worker runs after the sweep read the row but before the sweep
    # writes: run it inline from the sweep's enqueue. (The retry the
    # worker schedules goes through the configured job, not this stub,
    # so nothing recurses.)
    Agent::EventWebhookJob.expects(:perform_later).with do |event_id, _attempt|
      Agent::EventWebhookJob.perform_now(event_id)
      true
    end
    Agent::Delivery.recover_stranded_webhooks!

    assert_equal 1, event.reload.webhook_attempts
    assert_in_delta 600, event.reload.webhook_next_attempt_at - Time.current, 5
  end

  test "a recovery pass keeps going past an enqueue failure" do
    first = stranded_event(age: 5.minutes, attempts: 0)
    second = stranded_event(age: 6.minutes, attempts: 0)

    Agent::EventWebhookJob.expects(:perform_later).with(first.id, 0).raises(StandardError, "redis down")
    Agent::EventWebhookJob.expects(:perform_later).with(second.id, 0)

    # Calling it plainly proves the first row's failure never aborts the pass.
    Agent::Delivery.recover_stranded_webhooks!
  end

  test "a job with a stale attempt number exits without posting" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    event = stranded_event(age: 0.seconds, attempts: 1)
    event.update!(created_at: Time.current, webhook_next_attempt_at: Time.current)

    Agent::EventWebhookJob.perform_now(event.id, 0)

    assert_not_requested :post, webhooks(:bender).url
    assert_equal 1, event.reload.webhook_attempts
    assert_equal "pending", event.reload.webhook_status
  end

  test "a scheduled retry records its next attempt in the future" do
    WebMock.stub_request(:post, webhooks(:bender).url)
      .to_return(status: 429, headers: { "Retry-After" => "600" })

    event = stranded_event(age: 0.seconds, attempts: 0)
    event.update!(created_at: Time.current)
    Agent::EventWebhookJob.perform_now(event.id)

    assert_equal "pending", event.reload.webhook_status
    assert_in_delta 600, event.reload.webhook_next_attempt_at - Time.current, 5
  end

  test "the sweeper recovers a stranded pending delivery for a webhook-enabled agent" do
    assert webhooks(:bender).present?
    @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
      client_message_id: "strand-delivery-#{SecureRandom.hex(4)}"
    )
    event = @agent.agent_events.deliverable.last
    assert_equal "pending", event.outcome
    assert_equal "none", event.webhook_status
    event.update!(created_at: 3.minutes.ago)

    assert_enqueued_jobs 1, only: Agent::DeliveryJob do
      Agent::Delivery.recover_stranded_webhooks!
    end
  end

  test "the sweeper leaves a fresh pending delivery alone" do
    @room.messages.create!(
      creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
      client_message_id: "strand-fresh-#{SecureRandom.hex(4)}"
    )
    event = @agent.agent_events.deliverable.last
    event.update!(created_at: 1.minute.ago)

    assert_no_enqueued_jobs only: Agent::DeliveryJob do
      Agent::Delivery.recover_stranded_webhooks!
    end
  end

  private
    def stranded_event(age:, attempts:, webhook_status: "pending")
      @room.messages.create!(
        creator: users(:david), body: "Hey #{mention_attachment_for(:bender)}",
        client_message_id: "strand-#{SecureRandom.hex(4)}"
      )
      event = @agent.agent_events.deliverable.last
      event.update!(
        outcome: "delivered", webhook_status: webhook_status,
        webhook_attempts: attempts, created_at: age.ago
      )
      event
    end
end
