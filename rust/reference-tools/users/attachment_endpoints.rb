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
null_analysis = [[false,false],[true,false],[false,true]].map do |fail_analysis,fail_audit|
  Account.first.logo_attachment&.delete
  AuditLog.delete_all
  bytes = "plain logo fixture\n"
  blob = ActiveStorage::Blob.create_before_direct_upload!(filename:"notes.txt ",content_type:"application/octet-stream",byte_size:bytes.bytesize,checksum:Digest::MD5.base64digest(bytes))
  blob.upload_without_unfurling(StringIO.new(bytes))
  if fail_analysis
    ActiveRecord::Base.connection.execute("CREATE TRIGGER ws8br2_reject_null_analysis BEFORE UPDATE OF metadata ON active_storage_blobs WHEN json_extract(NEW.metadata,'$.analyzed')=1 BEGIN SELECT RAISE(ABORT,'deliberate null analysis failure'); END")
  end
  if fail_audit
    ActiveRecord::Base.connection.execute("CREATE TRIGGER ws8br2_reject_logo_audit BEFORE INSERT ON audit_logs WHEN NEW.action='account.settings.change' BEGIN SELECT RAISE(ABORT,'deliberate audit failure'); END")
  end
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  client.patch "/account",params:{account:{logo:blob.signed_id}},headers:{"X-CSRF-Token"=>token,"User-Agent"=>"ws8br2-attachment-fixture"}
  attached = Account.first.reload.logo.blob
  blob.reload
  audits = AuditLog.order(:id).map { |a| a.attributes.slice("action","actor_id","actor_label","target_type","target_id","target_label","details","ip_address","user_agent") }
  ActiveRecord::Base.connection.execute("DROP TRIGGER ws8br2_reject_null_analysis") if fail_analysis
  ActiveRecord::Base.connection.execute("DROP TRIGGER ws8br2_reject_logo_audit") if fail_audit
  {fail_analysis:fail_analysis,fail_audit:fail_audit,status:client.response.status,location:client.response.location,blob_reused:attached.id==blob.id,raw_filename:blob[:filename],sanitized:blob.filename.sanitized,content_type:blob.content_type,metadata:blob.metadata,analysis_jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.count { |j| j[:job]==ActiveStorage::AnalyzeJob },audits:audits}
end
puts JSON.pretty_generate(reference: "d7c7de92", rows: rows, null_analysis:null_analysis)
warn "Rails attachment endpoint oracle: #{rows.size} signed icon/logo assignments; #{null_analysis.size} after-commit NullAnalyzer responses and persisted snapshots; reference d7c7de92"
