require "test_helper"

class GithubPullRequestsHelperTest < ActionView::TestCase
  include Github::PullRequestsHelper

  class FailingQueueAdapter
    def initialize(error)
      @error = error
    end

    def enqueue(*)
      raise @error
    end

    def enqueue_at(*)
      raise @error
    end
  end

  test "cache key changes when a referenced pull request is updated" do
    message = messages(:first)
    pull_request = Github::PullRequest.create!(owner: "smart-data-ohio", repo: "smartfire", number: 42)
    Github::PullRequestReference.create!(message:, pull_request:)
    message.reload

    before = message_with_pr_cards_cache_key(message)
    travel 1.minute do
      pull_request.update!(title: "Updated title")
    end

    assert_not_equal before, message_with_pr_cards_cache_key(message.reload)
  end

  test "cache key for a message without pull requests is just the message" do
    assert_equal [ messages(:first), nil, nil ], message_with_pr_cards_cache_key(messages(:first))
  end

  test "cache key changes when the message is pinned and unpinned" do
    message = messages(:first)

    before = message_with_pr_cards_cache_key(message)

    travel 1.minute do
      MessagePin.pin!(message:, pinner: users(:david))
    end
    pinned_key = message_with_pr_cards_cache_key(message.reload)
    assert_not_equal before, pinned_key

    travel 1.minute do
      message.message_pins.sole.unpin!
    end
    assert_not_equal pinned_key, message_with_pr_cards_cache_key(message.reload)
  end

  test "cache key changes on unpin even when a referenced card is newer" do
    message = messages(:first)
    pull_request = Github::PullRequest.create!(owner: "smart-data-ohio", repo: "smartfire", number: 43)
    Github::PullRequestReference.create!(message:, pull_request:)
    MessagePin.pin!(message:, pinner: users(:david))

    pinned_key = message_with_pr_cards_cache_key(message.reload)

    travel 1.minute do
      pull_request.update!(title: "Updated title")
    end
    message.message_pins.sole.unpin!

    assert_not_equal pinned_key, message_with_pr_cards_cache_key(message.reload)
  end

  test "cache key changes when a referenced X post is fetched" do
    message = messages(:first)
    post = Twitter::Post.create!(post_id: "500", url: "https://x.com/jack/status/500")
    Twitter::PostReference.create!(message:, post:)
    message.reload

    before = message_with_pr_cards_cache_key(message)
    travel 1.minute do
      post.update!(text: "just setting up my twttr", fetched_at: Time.current)
    end

    assert_not_equal before, message_with_pr_cards_cache_key(message.reload)
  end

  test "cache key changes when a referenced event is updated" do
    message = messages(:first)
    event = events(:launch_party)
    EventReference.create!(message:, event:)
    message.reload

    before = message_with_pr_cards_cache_key(message)
    travel 1.minute do
      event.update!(title: "Updated title")
    end

    assert_not_equal before, message_with_pr_cards_cache_key(message.reload)
  end

  test "pr cards still render when the queue is down" do
    message = messages(:first)
    errors = [ Redis::BaseConnectionError.new("Redis down"), RedisClient::ConnectionError.new("Redis down") ]

    errors.each_with_index do |error, index|
      pull_request = Github::PullRequest.create!(owner: "example", repo: "app", number: 100 + index)
      Github::PullRequestReference.create!(message:, pull_request:)

      with_failing_queue_adapter(error) do
        assert_includes github_pr_cards_for(message.reload), pull_request
      end
    end
  end

  test "a failed pr enqueue releases its fetch claim" do
    message = messages(:first)
    pull_request = Github::PullRequest.create!(owner: "example", repo: "app", number: 500)
    Github::PullRequestReference.create!(message:, pull_request:)

    with_failing_queue_adapter(Redis::BaseConnectionError.new("Redis down")) do
      github_pr_cards_for(message.reload)
    end

    assert_nil pull_request.reload.fetch_requested_at
    assert pull_request.claim_fetch_request!
  end

  private
    def with_failing_queue_adapter(error)
      previous = ActiveJob::Base.queue_adapter
      ActiveJob::Base.queue_adapter = FailingQueueAdapter.new(error)
      yield
    ensure
      ActiveJob::Base.queue_adapter = previous
    end
end
