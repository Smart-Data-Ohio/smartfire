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
  "app/models/twitter/post_url.rb" => "9a3653ab31e1f5c1ce00e1185cd120d1024676291918a152b6dcbdc1229942cd",
  "lib/tasks/twitter.rake" => "03cc6417bc98144d907a545dbcc06c59ca18c906a2f017d0544df7a6f7c52284"
}.each { |file, hash| raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
travel_to Time.utc(2026, 3, 2, 16)
ActiveJob::Base.queue_adapter = :test
inputs = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/twitter_backfill_rendered_inputs.json")))
connection = ActiveRecord::Base.connection
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
def insert_input(row)
  connection = ActiveRecord::Base.connection
  connection.execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,forward_note,created_at,updated_at) VALUES(#{row.fetch('id')},486777696,127326141,#{connection.quote("backfill-#{row.fetch('id')}")},#{connection.quote(row['markdown'])},#{connection.quote(row['forward_note'])},'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  if row['html']
    connection.execute("INSERT INTO action_text_rich_texts(name,record_type,record_id,body,created_at,updated_at) VALUES('body','Message',#{row.fetch('id')},#{connection.quote(row['html'])},'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  end
end
def scenario(inputs)
  inputs.each { |row| insert_input(row) unless Message.exists?(id: row.fetch('id')) }
  projections = inputs.map do |row|
    message = Message.find(row.fetch('id'))
    selected_by_markdown = Twitter::PostUrl.post_url?(message.markdown_source)
    rendered = message.body.to_s unless selected_by_markdown
    canonical = message.body.body&.to_html
    row.merge(rendered: rendered, canonical: canonical, selected_by_markdown: selected_by_markdown,
      selected: selected_by_markdown || Twitter::PostUrl.post_url?(rendered),
      non_code_text: Twitter::PostUrl.non_code_text(canonical))
  end
  output = run_backfill
  actual = state
  repeated_output = run_backfill
  raise "non-idempotent operator" unless actual == state && output == repeated_output
  {inputs: projections, output: output, repeated_output: repeated_output, state: actual}
end
# Run the reviewer's exact single-row dataset first. It has no side effects of its
# own, so the same row can participate in the complete edge matrix afterwards.
single = scenario([inputs.find { |row| row.fetch('label') == 'hidden_attribute' }])
raise "single-row probe drift" unless single[:output] == "Backfilled 2 messages\n"
edges = scenario(inputs)
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA"), notes: "Unchanged Rails rendered-body selection; extraction uses canonical Content#to_html outside code, plus forward_note. Single-row review repro and 25 persisted-HTML edge cases use actual rake invocations twice, full reference/post/fetch-job state, and no body or state masks.", single: single, edges: edges)
warn "Rails Twitter rendered backfill: #{inputs.size} edge cases + exact single-row review repro; four actual rake invocations; repeated state unchanged"
