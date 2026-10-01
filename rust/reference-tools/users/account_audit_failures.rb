# Production HTTP requests deliberately outside a wrapping transaction: Rails saves first,
# then records the account audit. Observe both the response and committed attachment callbacks.
require "json"
require "nokogiri"
require "base64"
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.logger.level = Logger::FATAL
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = true
inputs = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "vectors/round_four_security.json")))
chrome = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
user = User.find(127326141)
session = user.sessions.detect(&:two_factor_verified?) || raise("no verified session")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
client = ActionDispatch::Integration::Session.new(Rails.application)
client.host! "campfire.test"
client.cookies[:session_token] = request.cookie_jar[:session_token]
client.get "/users/me/profile", headers: {"User-Agent" => chrome}
raise "initial page: #{client.response.status}" unless client.response.status == 200
token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')["content"]
headers = {"User-Agent" => chrome, "X-CSRF-Token" => token, "Accept" => "text/html"}
client.post "/sudo", params: {password: "secret123456"}, headers: headers
raise "sudo: #{client.response.status}" unless client.response.status == 302
rows = %w[settings join_reset styles logo_destroy settings_add_logo].to_h do |name|
  input = inputs.fetch("account").fetch(name)
  account = Account.first
  account.logo_attachment&.delete
  account.update_columns(name: "37signals", custom_styles: input["initial_styles"], settings: {restrict_room_creation_to_administrators: false})
  bytes = Base64.strict_decode64(inputs.fetch("logo_bytes"))
  account.logo.attach(io: StringIO.new(bytes), filename: "logo.png", content_type: "image/png") if input["initial_logo"]
  account.reload
  before_code = account.join_code
  AuditLog.delete_all
  ActiveRecord::Base.connection.execute("CREATE TRIGGER ws8br2_reject_account_audit BEFORE INSERT ON audit_logs BEGIN SELECT RAISE(ABORT,'review audit failure'); END")
  begin
    if input["upload"]
      file = Tempfile.new(["ws8br2-logo", ".png"])
      file.binmode
      file.write(bytes)
      file.rewind
      uploaded = Rack::Test::UploadedFile.new(file.path, "image/png", true, original_filename: "logo.png")
      client.patch "/account", params: {account: {logo: uploaded}}, headers: headers
      file.close!
    else
      client.public_send(input.fetch("method"), input.fetch("path"), params: {account: input.fetch("params")}, as: :json, headers: headers)
    end
    account.reload
    blob = account.logo.blob if account.logo.attached?
    [name, {status: client.response.status, location: client.response.location,
      name: account.name, styles: account.custom_styles, restrict: account.settings.restrict_room_creation_to_administrators?,
      code_changed: before_code != account.join_code, logo: account.logo.attached?,
      blob: blob&.attributes&.slice("filename", "content_type", "byte_size", "checksum", "metadata"),
      audits: AuditLog.order(:id).map { |a| a.attributes.slice("action", "actor_id", "actor_label", "target_type", "target_id", "target_label", "details", "ip_address", "user_agent") }}]
  ensure
    ActiveRecord::Base.connection.execute("DROP TRIGGER ws8br2_reject_account_audit")
  end
end
puts JSON.pretty_generate(reference: "d7c7de92", rows: rows)
warn "Rails account audit failure oracle: #{rows.size} production HTTP responses and committed account/blob snapshots; reference d7c7de92"
