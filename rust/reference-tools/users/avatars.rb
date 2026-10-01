require "json"
require "digest"
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/source-hashes.json"))).each do |file, hash|
  raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
Rails.application.config.hosts.clear
Rails.logger = ActiveSupport::Logger.new($stderr)
user = User.find(127326141)
session = ActionDispatch::Integration::Session.new(Rails.application)
session.host! "campfire.test"
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed.permanent[:session_token] = { value: user.sessions.first.token, httponly: true, same_site: :lax }
session.cookies["session_token"] = request.cookie_jar[:session_token]
user.avatar.destroy if user.avatar.attached?
names = ["David", "A B C", "Émile Smith", "A\u0301Bob", "𝟙Bob", "A-B <X>", "", "___ 42", "\u200dBob", "A\u203fBob", "²Bob", "\u0301Bob", "ⅠBob", "A\u200cBob", "A😀Bob"]
cases = names.map do |name|
  user.update!(name: name)
  session.get "/users/#{user.avatar_token}/avatar", headers: { "Accept" => "image/svg+xml" }
  ActiveSupport::IsolatedExecutionState.clear
  raise "avatar status: #{session.response.status}" unless session.response.status == 200
  { name: name, initials: user.initials, body: session.response.body, etag: session.response.headers["ETag"], cache_control: session.response.headers["Cache-Control"] }
end
puts JSON.pretty_generate(reference: "d7c7de92", user_id: user.id, cases: cases)
warn "Rails avatar oracle: #{cases.length} initials SVG bodies; reference d7c7de92"
