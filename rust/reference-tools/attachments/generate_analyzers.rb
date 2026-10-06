# Run on our pinned Rails default seed, with the frozen parity clock.
# Observe the attachment's after_create_commit callback with real admin auth and CSRF.
require "json"
require "base64"
require "digest"
Rails.logger = Logger.new($stderr)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = true
work = ENV.fetch("PARITY_WORK")
signed_vectors = JSON.parse(File.read(File.join(work, "vectors/attachment_assignments.json")))
inputs = [
  {kind: "text", filename: "analysis.txt", content_type: "text/plain", data_base64: Base64.strict_encode64("Attachment analyzer parity.\n")},
  {kind: "image", filename: "analysis.png", content_type: "image/png", data_base64: signed_vectors.fetch("png_base64")},
  {kind: "video", filename: "alpha-centuri.mov", content_type: "video/quicktime", reference_fixture: "test/fixtures/files/alpha-centuri.mov"},
  {kind: "audio", filename: "analysis.wav", content_type: "audio/wav", rust_fixture: "test-support/attachments/analysis.wav"},
  {kind: "pdf", filename: "analysis.pdf", content_type: "application/pdf", rust_fixture: "test-support/attachments/analysis.pdf"}
]
user = User.find(127326141)
session = user.sessions.detect(&:two_factor_verified?) || raise("no verified session")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
client = ActionDispatch::Integration::Session.new(Rails.application)
client.host! "campfire.test"
client.cookies["session_token"] = request.cookie_jar[:session_token]
client.get "/account/edit"
raise "not authenticated" unless client.response.status == 200
token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')["content"]
cases = inputs.map do |input|
  bytes = if input[:reference_fixture]
    File.binread(Rails.root.join(input[:reference_fixture]))
  elsif input[:rust_fixture]
    File.binread(File.join(work, input[:rust_fixture]))
  else
    Base64.strict_decode64(input.fetch(:data_base64))
  end
  blob = ActiveStorage::Blob.create_before_direct_upload!(filename: input[:filename], content_type: input[:content_type], byte_size: bytes.bytesize, checksum: Digest::MD5.base64digest(bytes))
  blob.upload_without_unfurling(StringIO.new(bytes))
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  client.patch "/account", params: {account: {logo: blob.signed_id}}, headers: {"X-CSRF-Token" => token}
  raise "assignment failed: #{input[:kind]} #{client.response.status}" unless client.response.status == 302
  raise "attachment missing" unless Account.first.logo.blob.id == blob.id
  blob.reload
  analyzer = blob.send(:analyzer_class)
  jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.count { |job| job[:job] == ActiveStorage::AnalyzeJob }
  input.merge(sha256: Digest::SHA256.hexdigest(bytes), status: client.response.status,
    identified_content_type: blob.content_type, analyzer: analyzer.name,
    analyze_later: analyzer.analyze_later?, analysis_jobs: jobs, metadata: blob.metadata)
end
# A failing synchronous analyzer must leave the parent and attachment committed.
bytes = Base64.strict_decode64(inputs.first.fetch(:data_base64))
blob = ActiveStorage::Blob.create_before_direct_upload!(filename: "failure.txt", content_type: "text/plain", byte_size: bytes.bytesize, checksum: Digest::MD5.base64digest(bytes))
blob.upload_without_unfurling(StringIO.new(bytes))
ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_null_analysis BEFORE UPDATE OF metadata ON active_storage_blobs WHEN json_extract(NEW.metadata, '$.analyzed') = 1 BEGIN SELECT RAISE(ABORT, 'reject inline analysis'); END;")
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
client.patch "/account", params: {account: {name: "Null analyzer committed", logo: blob.signed_id}}, headers: {"X-CSRF-Token" => token}
failure = {status: client.response.status, account_name: Account.first.name,
  attachment_committed: Account.first.logo.blob.id == blob.id, metadata: blob.reload.metadata,
  analysis_jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.count { |job| job[:job] == ActiveStorage::AnalyzeJob }}
raise "wrong failure boundary" unless failure[:status] == 500 && failure[:attachment_committed]
puts JSON.pretty_generate(reference: File.read(File.join(work, "parity/reference.sha")).strip,
  now: Time.current.iso8601, cases: cases, inline_failure: failure)
