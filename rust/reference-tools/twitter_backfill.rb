# Fresh execution of the operator rake task; fixture writes bypass message-save syncs.
require "json"
require "digest"
require "rake"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
Rails.logger = Logger.new($stderr, level: Logger::WARN)
{
  "app/models/twitter/post_reference_backfill.rb" => "41410ff35a9544505b07d1e0db4977180e6fa27a47d02cd7ceb9503050b815bd",
  "app/models/twitter/post_reference_sync.rb" => "38ee493e470472b299cdcab85977a530fda163341b035b6a45c2c02103802150",
  "lib/tasks/twitter.rake" => "03cc6417bc98144d907a545dbcc06c59ca18c906a2f017d0544df7a6f7c52284"
}.each { |file, hash| raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
travel_to Time.utc(2026, 3, 2, 16)
ActiveJob::Base.queue_adapter = :test
inputs = [
  {id: -2, markdown: "https://x.com/negative/status/77101", html: "<p>https://x.com/negative/status/77101</p>"},
  {id: 0, markdown: "https://twitter.com/zero/statuses/77102", html: "<p>https://twitter.com/zero/statuses/77102</p>"},
  {id: 2118001001, markdown: "https://x.com/code/status/77103", html: "<pre>https://x.com/code/status/77103</pre>"},
  {id: 2118001002, markdown: "https://x.com/missing/status/77104", html: nil},
  {id: 2118001003, markdown: nil, html: '<p><a href="https://mobile.twitter.com/legacy/status/77105?s=20">legacy</a></p>'},
  {id: 2118001004, markdown: "ordinary", html: "<p>ordinary</p>", forward_note: "https://x.com/note/status/77106"},
  {id: 2118001005, markdown: "https://x.com/dupe/status/77101", html: "<p>https://x.com/dupe/status/77101 https://x.com/a/status/77107 https://x.com/a/status/77108 https://x.com/a/status/77109 https://x.com/a/status/77110</p>"}
]
filler_count = 1001
connection = ActiveRecord::Base.connection
rows = (0...filler_count).map { |n| {id: 2118000000+n, markdown: "ordinary", html: nil} } + inputs
rows.each do |row|
  connection.execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,forward_note,created_at,updated_at) VALUES(#{row[:id]},486777696,127326141,#{connection.quote("backfill-#{row[:id]}")},#{connection.quote(row[:markdown])},#{connection.quote(row[:forward_note])},'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  if row[:html]
    connection.execute("INSERT INTO action_text_rich_texts(name,record_type,record_id,body,created_at,updated_at) VALUES('body','Message',#{row[:id]},#{connection.quote(row[:html])},'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  end
end
Rails.application.load_tasks
def run_backfill
  task = Rake::Task["twitter:backfill_references"]
  task.reenable
  original = $stdout
  $stdout = StringIO.new
  task.invoke
  $stdout.string
ensure
  $stdout = original
end
def state
  {references: Twitter::PostReference.joins(:post).order(:message_id, "twitter_posts.post_id").pluck(:message_id, "twitter_posts.post_id"),
   posts: Twitter::Post.order(:post_id).pluck(:post_id, :url, :fetch_requested_at).map { |id, url, at| [id, url, at&.utc&.iso8601(6)] },
   jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.select { |job| job[:job] == Twitter::FetchPostJob }.map { |job| GlobalID::Locator.locate(job[:args].first.fetch("_aj_globalid")).post_id }.sort}
end
first = run_backfill
first_state = state
second = run_backfill
raise "non-idempotent operator" unless first_state == state
puts JSON.pretty_generate(reference: "d7c7de92", filler_count: filler_count, inputs: inputs, output: first, repeated_output: second, state: first_state)
warn "Rails Twitter backfill: 1008 inserted rows; two actual rake invocations; identical repeated state"
