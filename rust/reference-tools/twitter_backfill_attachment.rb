# Saved HTML ContentAttachment failure, through the real rake and sync producers.
require "json"
require "digest"
require "rake"
require "open3"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
Rails.logger = Logger.new($stderr, level: Logger::WARN)
travel_to Time.utc(2026, 3, 2, 16)
ActiveJob::Base.queue_adapter = :test
Rails.application.load_tasks
sources = %w[app/models/twitter/post_reference_backfill.rb app/models/twitter/post_reference_sync.rb
  app/models/twitter/post_url.rb lib/tasks/twitter.rake]
source_hashes = sources.to_h { |file| [file, Digest::SHA256.file(Rails.root.join(file)).hexdigest] }
connection = ActiveRecord::Base.connection
html = '<action-text-attachment content-type="text/html" content="&lt;p&gt;https&amp;#58;//x.com/a/status/99013&lt;/p&gt;"></action-text-attachment>'
inputs = [
  {id: 2120000007, markdown: "https://x.com/before/status/99112", html: "<p>https://x.com/before/status/99112</p>"},
  {id: 2120000008, markdown: nil, html: html, forward_note: "https://x.com/note/status/99113"},
  {id: 2120000009, markdown: "https://x.com/after/status/99114", html: "<p>https://x.com/after/status/99114</p>"}
]
inputs.each do |row|
  connection.execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,forward_note,created_at,updated_at) VALUES(#{row[:id]},486777696,127326141,#{connection.quote("backfill-#{row[:id]}")},#{connection.quote(row[:markdown])},#{connection.quote(row[:forward_note])},'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  # Normal saved-content roundtrip, including canonicalization and rich-text callbacks.
  ActionText::RichText.create!(name: "body", record_type: "Message", record_id: row[:id], body: row[:html])
end
stored = inputs.map { |row| row.merge(html: Message.find(row[:id]).body.body.to_html) }
def state
  {references: Twitter::PostReference.joins(:post).order(:message_id, "twitter_posts.post_id").pluck(:message_id, "twitter_posts.post_id"),
   posts: Twitter::Post.order(:post_id).pluck(:post_id, :url, :fetch_requested_at).map { |id, url, at| [id, url, at&.utc&.iso8601(6)] },
   jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.select { |job| job[:job] == Twitter::FetchPostJob }.map { |job| GlobalID::Locator.locate(job[:args].first.fetch("_aj_globalid")).post_id }.sort}
end
def attempt
  original = $stdout
  $stdout = StringIO.new
  task = Rake::Task["twitter:backfill_references"]
  task.reenable
  task.invoke
  raise "expected ContentAttachment rendering failure"
rescue ActionView::Template::Error => error
  {stdout: $stdout.string, error: {class: error.class.name, message: error.message},
   cause: {class: error.cause.class.name, message: error.cause.message},
   open_transactions: ActiveRecord::Base.connection.open_transactions, state: state}
ensure
  $stdout = original
end
first = attempt
second = attempt
raise "repeat changed persisted state" unless first == second
cli_stdout, cli_stderr, cli_status = Open3.capture3("bin/rails", "twitter:backfill_references")
raise "rake CLI exit/output drift" unless cli_status.exitstatus == 1 && cli_stdout.empty? &&
  cli_stderr.include?("ActionView::Template::Error: undefined method 'include?' for nil")
raise "CLI abort changed rows" unless first == attempt
cli = {exit_status: cli_status.exitstatus, stdout: cli_stdout, stderr: cli_stderr}
message = Message.find(2120000008)
begin
  rendered = message.body.to_s
rescue => error
  render_error = {class: error.class.name, message: error.message}
end
# Request rendering uses the real MessagesController renderer, whose context
# prefix is present. It must continue rendering ContentAttachments successfully.
controller = MessagesController.new
controller.set_request!(ActionDispatch::Request.new(Rack::MockRequest.env_for("http://campfire.test/")))
controller.set_response!(ActionDispatch::Response.new)
request_rendered = ActionText::Content.with_renderer(controller) { message.body.to_s }
Twitter::PostReferenceSync.call(message)
synced = state
Twitter::PostReferenceSync.call(message)
raise "sync repeat changed persisted state" unless state == synced
# Markdown selection short-circuits rendering, including the failing attachment.
message.update_columns(markdown_source: "https://x.com/source/status/99115")
original = $stdout
$stdout = StringIO.new
Rake::Task["twitter:backfill_references"].reenable
Rake::Task["twitter:backfill_references"].invoke
short_circuit = {stdout: $stdout.string, state: state}
$stdout = original
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA"), source_hashes: source_hashes,
  notes: "Normally saved HTML content attachment. The default ApplicationController renderer raises because the nested object renderer has a nil context prefix; request rendering succeeds. No whole-run transaction: earlier syncs and fetch jobs remain, the failing message and later rows are untouched. Direct sync only canonicalizes and succeeds. Four actual rake aborts (three in-process and one real CLI) and one successful Markdown short-circuit invocation.",
  inputs: inputs, stored: stored, first: first, repeated: second, cli: cli, render_error: render_error,
  request_rendered: request_rendered, sync: synced, short_circuit: short_circuit)
warn "Rails Twitter content attachment: normal save; four rake aborts including real CLI; zero open transactions; direct sync + repeat; request renderer; Markdown short-circuit"
