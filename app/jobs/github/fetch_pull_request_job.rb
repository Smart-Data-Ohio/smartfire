class Github::FetchPullRequestJob < ApplicationJob
  # The fetcher records every failure on the PR instead of raising, so a
  # missing record is the only thing left to discard: the PR was deleted
  # after the job was enqueued and there is nothing to update.
  discard_on ActiveJob::DeserializationError

  # The fetcher records every failure on the PR instead of raising, so there
  # is nothing a retry would fix. attempts: 1 opts out of the inherited
  # transient retries.
  retry_on(*ApplicationJob::TRANSIENT_ERRORS, attempts: 1)

  def perform(pull_request)
    Github::PullRequestFetcher.new(pull_request).fetch
  end
end
