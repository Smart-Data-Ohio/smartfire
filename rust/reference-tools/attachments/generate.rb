# Run at the pinned Rails revision on the default seed and frozen parity clock.
# Produces signed-ID assignment vectors and request-level differentials with real CSRF.
require "json"
require "base64"
require "vips"
Rails.logger = Logger.new($stderr)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = true
now = Time.current
png = Vips::Image.black(64, 64).pngsave_buffer
blob = ActiveStorage::Blob.create_before_direct_upload!(filename: "attachment-parity.png", content_type: "image/png", byte_size: png.bytesize, checksum: Digest::MD5.base64digest(png))
blob.upload_without_unfurling(StringIO.new(png))
verifier = ActiveStorage.verifier
valid = blob.signed_id
cases = [
  ["valid", valid],
  ["before expiry", blob.signed_id(expires_at: now + 60)],
  ["at expiry", blob.signed_id(expires_at: now)],
  ["expired", blob.signed_id(expires_at: now - 1)],
  ["wrong purpose", verifier.generate(blob.id, purpose: :variation)],
  ["tampered digest", valid[0...-1] + (valid[-1] == "0" ? "1" : "0")],
  ["tampered payload", (valid[0] == "A" ? "B" : "A") + valid[1..]],
  ["missing blob", verifier.generate(9_000_000_000, purpose: :blob_id)]
]
user = User.find(127326141)
session = user.sessions.detect(&:two_factor_verified?) || raise("no verified session")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
cookie = request.cookie_jar[:session_token]
rows = cases.map.with_index do |(label, signed), index|
  verified = verifier.verified(signed, purpose: :blob_id)
  statuses = {}
  [["logo", :patch, "/account", {account: {logo: signed}}],
   ["icon", :post, "/account/icons", {workspace_icon: {name: "attach_vector_#{index}", title: "Attachment parity", image: signed}}]].each do |kind, method, path, params|
    client = ActionDispatch::Integration::Session.new(Rails.application)
    client.host! "campfire.test"
    client.cookies["session_token"] = cookie
    client.get "/account/edit"
    raise "not authenticated: #{client.response.status}" unless client.response.status == 200
    token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')["content"]
    client.public_send(method, path, params: params, headers: {"X-CSRF-Token" => token})
    statuses[kind] = client.response.status
  end
  {case: label, signed_id: signed, expected_id: verified, statuses: statuses}
end
# Callback differential: attaching an unanalyzed image schedules analysis; reassigning it
# after analysis preserves the attachment and adds no analysis job.
blob.reload.update!(metadata: {identified: true})
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
user.avatar.attach(blob)
analysis_count = -> { ActiveJob::Base.queue_adapter.enqueued_jobs.count { |j| j[:job] == ActiveStorage::AnalyzeJob } }
first_jobs = analysis_count.call
attachment_id = user.avatar.attachment.id
blob.analyze
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
user.avatar.attach(blob.signed_id)
raise "same-blob reassignment replaced attachment" unless user.avatar.attachment.id == attachment_id
puts JSON.pretty_generate(reference: ENV.fetch("GIT_REVISION"), now: now.iso8601, blob_id: blob.id,
  png_base64: Base64.strict_encode64(png), cases: rows,
  analysis: {first_attachment_jobs: first_jobs, analyzed_reassignment_jobs: analysis_count.call,
    metadata: blob.reload.metadata, same_attachment: true})
