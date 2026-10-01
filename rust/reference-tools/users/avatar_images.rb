# The two remaining Users::AvatarsControllerTest cases at d7c7de92, through real uploads.
require "json"
require "base64"
require "digest"
Rails.application.config.hosts.clear
Rails.logger = ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/source-hashes.json"))).each do |file, hash|
  raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
user = User.find(127326141)
session = ActionDispatch::Integration::Session.new(Rails.application)
session.host! "campfire.test"
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed.permanent[:session_token] = { value: user.sessions.first.token, httponly: true, same_site: :lax }
session.cookies["session_token"] = request.cookie_jar[:session_token]
session.get "/users/me/profile"
ActiveSupport::IsolatedExecutionState.clear
csrf = Nokogiri::HTML(session.response.body).at_css('meta[name="csrf-token"]')["content"]
cases = %w[moon.jpg pixel.bmp].map do |file|
  type = file.end_with?("jpg") ? "image/jpeg" : "image/bmp"
  path = Rails.root.join("test/fixtures/files", file)
  session.patch "/users/me/profile", params: { user: { avatar: Rack::Test::UploadedFile.new(path, type) } }, headers: { "X-CSRF-Token" => csrf }
  ActiveSupport::IsolatedExecutionState.clear
  raise "avatar upload #{file}: #{session.response.status}" unless session.response.status == 302
  session.get "/users/#{user.reload.avatar_token}/avatar", headers: { "Accept" => "image/svg+xml" }
  ActiveSupport::IsolatedExecutionState.clear
  raise "avatar show #{file}: #{session.response.status}" unless session.response.status == 200
  shown = { file: file, input_sha256: Digest::SHA256.file(path).hexdigest, status: session.response.status,
    body: Base64.strict_encode64(session.response.body), content_type: session.response.headers["Content-Type"],
    cache_control: session.response.headers["Cache-Control"], etag: session.response.headers["ETag"] }
  session.get "/users/#{user.avatar_token}/avatar", headers: { "Accept" => "image/svg+xml", "If-None-Match" => shown[:etag] }
  ActiveSupport::IsolatedExecutionState.clear
  shown.merge!(fresh_status: session.response.status, fresh_body: Base64.strict_encode64(session.response.body), fresh_etag: session.response.headers["ETag"])
  session.get "/users/#{user.avatar_token}/avatar", headers: { "Accept" => "image/svg+xml", "If-None-Match" => shown[:fresh_etag] }
  ActiveSupport::IsolatedExecutionState.clear
  shown.merge(stable_status: session.response.status, stable_body: Base64.strict_encode64(session.response.body))
end
puts JSON.pretty_generate(reference: "d7c7de92", vips: Vips.version_string, cases: cases)
warn "Rails avatar images oracle: 2 real uploads, complete WebP and fallback SVG bodies with cache headers; libvips #{Vips.version_string}; reference d7c7de92"
