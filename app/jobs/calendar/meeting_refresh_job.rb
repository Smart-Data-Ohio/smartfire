class Calendar::MeetingRefreshJob < ApplicationJob
  # Idempotent: re-reads the member's upcoming event times and replaces
  # the cached intervals. Google failures are recorded on the cache row
  # (status reads as off) instead of retrying: the 15-minute refresh
  # cadence is the retry.
  def perform(user_id)
    Calendar::MeetingRefresh.refresh(user_id)
  end
end
