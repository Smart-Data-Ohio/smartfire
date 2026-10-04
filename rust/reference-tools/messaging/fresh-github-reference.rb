require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
# Record enqueues without executing any network job.
ActiveJob::Base.queue_adapter = :test
card = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 3141)
card.update!(private: false, title: "Cached card", state: "open", fetched_at: Time.current, fetch_requested_at: nil)
message = Room.find(486777696).root_messages.create!(creator: User.find(127326141), markdown_source: "see https://github.com/rails/rails/pull/3141", client_message_id: "fresh-github-reference")
count = -> { ActiveJob::Base.queue_adapter.enqueued_jobs.count { |job| job[:job] == Github::FetchPullRequestJob } }
rows = [{ name: "new_reference", stale: card.reload.stale?, fetches: count.call }]
card.update_columns(fetch_requested_at: nil)
Github::PullRequestReferenceSync.call(message.reload)
rows << { name: "existing_reference", stale: card.reload.stale?, fetches: count.call }
raise "unexpected fresh reference behavior" unless rows.map { |row| [row[:stale], row[:fetches]] } == [[false, 1], [false, 1]]
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], rows:) + "\n")
puts "WS8bm fresh GitHub reference: Rails enqueues the new reference; unchanged fresh reference enqueues nothing"
