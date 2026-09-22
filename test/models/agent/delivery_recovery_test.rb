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

  test "an exhausted pending row is left alone" do
    stranded_event(age: 10.minutes, attempts: Agent::EventWebhookJob::MAX_ATTEMPTS)

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end
  end

  test "settled webhook rows are left alone" do
    stranded_event(age: 10.minutes, attempts: 1, webhook_status: "delivered")
    stranded_event(age: 10.minutes, attempts: 5, webhook_status: "failed")

    assert_no_enqueued_jobs only: Agent::EventWebhookJob do
      Agent::Delivery.recover_stranded_webhooks!
    end
  end

  test "a recovery pass keeps going past an enqueue failure" do
    first = stranded_event(age: 5.minutes, attempts: 0)
    second = stranded_event(age: 6.minutes, attempts: 0)

    Agent::EventWebhookJob.expects(:perform_later).with(first.id).raises(StandardError, "redis down")
    Agent::EventWebhookJob.expects(:perform_later).with(second.id)

    # Calling it plainly proves the first row's failure never aborts the pass.
    Agent::Delivery.recover_stranded_webhooks!
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
