# Run on our pinned Rails default seed, with the frozen parity clock.
# Both Blob::Identifiable and Blob#unfurl use Filename#to_s, while the row keeps the raw string.
require "json"
require "base64"
require "digest"
Rails.logger = Logger.new($stderr)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
work = ENV.fetch("PARITY_WORK")
png = JSON.parse(File.read(File.join(work, "vectors/attachment_assignments.json"))).fetch("png_base64")
names = [
  "file.png/", "file.png ", "file.png//", "file.png\\", "file.png /", " file.png ",
  "file.png", "file.PNG", "file", "file.", ".png", "dir/file.png", "dir\\file.png",
  "../file.png", "/file.png", "C:\\dir\\file.png", "dir.png/file", "dir.png\\file",
  "file.png/next", "file.png\\next", "file.png\t", "file.png\r\n", "file.png\v\f",
  "\0file.png\0", "file.png\u007f", "file.png\u0085", "file.png\u00a0",
  "file.png\u2003", "file.png\u2028", "file.png\u2029", "file.png\u3000",
  "file.png\u202e", "\u202efile.png", "file\u202e.png", "café.png ", "cafe\u0301.png/",
  "日本語.png ", "📎.png/", "file.png／", "file.png∕", "file.png＼", "a%$|:;<>?*\".png",
  "a%$|:;<>?*\".png/", "a b.png ", "a.b/", "a.b\\", "file.wav ", "file.mp4/", "file.pdf "
]
# All ASCII controls except interior NUL (which Ruby File.extname rejects); NUL stripping is above.
(1..31).each do |code|
  names << "file.png#{code.chr}" << "file#{code.chr}.png" << "#{code.chr}file.png"
end
names.uniq!
inputs = [
  {kind: "hello", data_base64: Base64.strict_encode64("hello")},
  {kind: "empty", data_base64: ""},
  {kind: "png", data_base64: png},
  {kind: "pdf", data_base64: Base64.strict_encode64(File.binread(File.join(work, "reference-tools/attachments/fixtures/analysis.pdf")))}
]
def direct_blob(name, bytes)
  blob = ActiveStorage::Blob.create_before_direct_upload!(filename: name, content_type: "application/octet-stream",
    byte_size: bytes.bytesize, checksum: Digest::MD5.base64digest(bytes))
  blob.upload_without_unfurling(StringIO.new(bytes))
  blob
end
cases = names.flat_map do |name|
  inputs.map do |input|
    bytes = Base64.strict_decode64(input.fetch(:data_base64))
    blob = direct_blob(name, bytes)
    stored_before = blob.reload[:filename]
    blob.identify_without_saving
    blob.save!
    blob.reload
    uploaded = ActiveStorage::Blob.create_and_upload!(io: StringIO.new(bytes), filename: name, content_type: "application/octet-stream")
    raise "identification differs from unfurl" unless blob.content_type == uploaded.content_type
    input.merge(filename: name, sanitized: blob.filename.sanitized, stored_before: stored_before,
      stored_after: blob[:filename], uploaded_filename: uploaded.reload[:filename],
      content_type: "application/octet-stream", identified_content_type: blob.content_type,
      metadata: blob.metadata, uploaded_metadata: uploaded.metadata, analyzer: blob.send(:analyzer_class).name,
      analyze_later: blob.send(:analyzer_class).analyze_later?)
  end
end

# Exercise the opposite analyzer mismatches through real admin requests with CSRF enabled.
ActionController::Base.allow_forgery_protection = true
session = User.find(127326141).sessions.detect(&:two_factor_verified?) || raise("no verified session")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
client = ActionDispatch::Integration::Session.new(Rails.application)
client.host! "campfire.test"
client.cookies["session_token"] = request.cookie_jar[:session_token]
client.get "/account/edit"
raise "not authenticated" unless client.response.status == 200
token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')["content"]
assignments = ["file.png/", "file.png "].each_with_index.map do |name, index|
  bytes = "hello"
  blob = direct_blob(name, bytes)
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  client.patch "/account", params: {account: {logo: blob.signed_id}}, headers: {"X-CSRF-Token" => token}
  raise "assignment failed" unless client.response.status == 302 && Account.first.logo.blob.id == blob.id
  blob.reload
  {kind: index == 0 ? "filename_slash" : "filename_space", filename: name, data_base64: Base64.strict_encode64(bytes),
    sha256: Digest::SHA256.hexdigest(bytes), content_type: "application/octet-stream", status: client.response.status,
    stored_filename: blob[:filename], sanitized: blob.filename.sanitized, identified_content_type: blob.content_type,
    analyzer: blob.send(:analyzer_class).name, metadata: blob.metadata,
    analysis_jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.count { |job| job[:job] == ActiveStorage::AnalyzeJob }}
end
puts JSON.pretty_generate(reference: File.read(File.join(work, "parity/reference.sha")).strip,
  now: Time.current.iso8601, filename_count: names.size, cases: cases, assignments: assignments)
