# Actual signed-blob assignments through our pinned Rails controllers, with CSRF enabled.
require "json"
require "digest"
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = true
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
rows = []
["logo", "icon"].each do |kind|
  ["clean.svg", "square_64.png"].each do |fixture|
    [" ", "/"].each_with_index do |suffix, index|
      name = "ws8br2_#{kind}_#{fixture.tr('.', '_')}_#{index}"
      bytes = File.binread(File.join(ENV.fetch("PARITY_WORK"), "vectors/workspace_icons", fixture))
      filename = fixture + suffix
      blob = ActiveStorage::Blob.create_before_direct_upload!(filename: filename, content_type: "application/octet-stream", byte_size: bytes.bytesize, checksum: Digest::MD5.base64digest(bytes))
      blob.upload_without_unfurling(StringIO.new(bytes))
      ActiveJob::Base.queue_adapter.enqueued_jobs.clear
      before_audits = AuditLog.count
      if kind == "logo"
        client.patch "/account", params: {account: {logo: blob.signed_id}}, headers: {"X-CSRF-Token" => token}
        attached = Account.first.reload.logo.blob
        errors = []
      else
        client.post "/account/icons", params: {workspace_icon: {name: name, title: "Signed blob", image: blob.signed_id}}, headers: {"X-CSRF-Token" => token}
        icon = WorkspaceIcon.find_by(name: name)
        attached = icon&.image&.blob
        rejected = WorkspaceIcon.new(name: name, title: "Signed blob", creator: user, image: blob.signed_id)
        rejected.valid?
        errors = rejected.errors.full_messages unless icon
        errors ||= []
      end
      blob.reload
      rows << {kind: kind, fixture: fixture, filename: filename, name: name, status: client.response.status,
        location: client.response.location, attached: attached&.id == blob.id, raw_filename: blob[:filename], sanitized: blob.filename.sanitized,
        content_type: blob.content_type, metadata: blob.metadata, errors: errors,
        analysis_jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.count { |j| j[:job] == ActiveStorage::AnalyzeJob }, audits: AuditLog.count - before_audits}
    end
  end
end
puts JSON.pretty_generate(reference: "d7c7de92", rows: rows)
warn "Rails attachment endpoint oracle: #{rows.size} signed icon/logo assignments; reference d7c7de92"
