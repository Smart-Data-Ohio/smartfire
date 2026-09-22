require "test_helper"

class ApplicationJobTest < ActiveJob::TestCase
  class TransientFailureJob < ApplicationJob
    def perform
      raise Net::ReadTimeout, "boom"
    end
  end

  test "retries transient errors instead of raising" do
    assert_enqueued_with(job: TransientFailureJob) do
      TransientFailureJob.perform_now
    end
  end

  test "retries each listed transient error" do
    ApplicationJob::TRANSIENT_ERRORS.each do |error_class|
      job_class = Class.new(ApplicationJob) do
        define_method(:perform) { raise error_class, "boom" }
      end

      before = enqueued_jobs.size
      job_class.perform_now
      assert_equal before + 1, enqueued_jobs.size, "#{error_class} should retry"
    end
  end

  test "discards jobs whose records are gone" do
    room = rooms(:designers)
    message = room.root_messages.create!(creator: users(:david), markdown_source: "gone soon")

    Room::PushMessageJob.perform_later(room, message)
    message.destroy!

    assert_nothing_raised do
      perform_enqueued_jobs
    end
  end

  test "a job that keeps the defaults retries on transient failure" do
    room = rooms(:designers)
    message = messages(:first)

    Room::MessagePusher.stubs(:new).raises(Net::ReadTimeout, "push timed out")

    assert_enqueued_with(job: Room::PushMessageJob) do
      Room::PushMessageJob.perform_now(room, message)
    end
  end

  test "retries are scheduled with polynomial backoff" do
    travel_to Time.current do
      assert_enqueued_with(job: TransientFailureJob) do
        TransientFailureJob.perform_now
      end
      first_at = enqueued_jobs.last[:at]

      perform_enqueued_jobs
      second_at = enqueued_jobs.last[:at]

      assert_not_nil first_at
      assert_operator second_at - first_at, :>, 5, "expected the backoff to grow between attempts"
    end
  end

  test "Agent::DeliveryJob does not retry" do
    agent = agents(:bender_agent)
    event = agent.agent_events.create!(event_type: "mention", outcome: "pending")
    Agent::Delivery.stubs(:perform).raises(Net::ReadTimeout, "boom")

    assert_raises(Net::ReadTimeout) { Agent::DeliveryJob.perform_now(event.id) }
    assert_no_enqueued_jobs
  end

  test "Calendar::SyncEntryJob retries transient Google failures with a delay" do
    Calendar::EntrySync.stubs(:sync).raises(Google::Client::Unavailable, "boom")

    assert_enqueued_with(job: Calendar::SyncEntryJob) { Calendar::SyncEntryJob.perform_now(1, 1) }
  end

  test "Github::FetchPullRequestJob does not retry" do
    pull_request = Github::PullRequest.create!(owner: "rails", repo: "rails", number: 1)
    Github::PullRequestFetcher.any_instance.stubs(:fetch).raises(Net::ReadTimeout, "boom")

    assert_raises(Net::ReadTimeout) { Github::FetchPullRequestJob.perform_now(pull_request) }
    assert_no_enqueued_jobs
  end

  test "Twitter::FetchPostJob does not retry" do
    Twitter::PostFetcher.any_instance.stubs(:fetch).raises(Net::ReadTimeout, "boom")

    assert_raises(Net::ReadTimeout) { Twitter::FetchPostJob.perform_now(twitter_posts(:jack_first_post)) }
    assert_no_enqueued_jobs
  end

  test "Github::PerformAgentActionJob does not retry" do
    AgentApproval.stubs(:find_by).raises(Net::ReadTimeout, "boom")

    assert_raises(Net::ReadTimeout) { Github::PerformAgentActionJob.perform_now(1) }
    assert_no_enqueued_jobs
  end

  test "Huddle::CleanupJob does not retry" do
    HuddleCleanup.stubs(:find_by).raises(Net::ReadTimeout, "boom")

    assert_raises(Net::ReadTimeout) { Huddle::CleanupJob.perform_now(1) }
    assert_no_enqueued_jobs
  end
end
