# HTTP and policy vectors from our pinned Rails app. No app callbacks or helpers are stubbed.
require "json"
require "base64"
require "digest"
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/source-hashes.json"))).each do |file, hash|
  raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
Rails.logger = ActiveSupport::Logger.new($stderr)

Rails.application.config.hosts.clear
keys = %w[LEGAL_OPERATOR_NAME LEGAL_CONTACT_EMAIL LEGAL_EFFECTIVE_DATE]
states = {
  default: {},
  original_configured: {"LEGAL_OPERATOR_NAME" => "Acme Widgets", "LEGAL_CONTACT_EMAIL" => "privacy@example.com"},
  configured: { "LEGAL_OPERATOR_NAME" => "  Acme & Widgets  ", "LEGAL_CONTACT_EMAIL" => " privacy+chat@example.test ", "LEGAL_EFFECTIVE_DATE" => " October 1, 2026 " },
  hostile: { "LEGAL_OPERATOR_NAME" => "<script>alert(1)</script>", "LEGAL_CONTACT_EMAIL" => 'privacy@example.test"><script>alert(1)</script>', "LEGAL_EFFECTIVE_DATE" => "<script>today</script>" },
  unicode: { "LEGAL_OPERATOR_NAME" => "\u00a0Org\u00a0", "LEGAL_CONTACT_EMAIL" => "\u00a0mail@example.test\u00a0", "LEGAL_EFFECTIVE_DATE" => "Été 2026" },
  mail_uri: { "LEGAL_CONTACT_EMAIL" => "a!#$%&'*+/=?^_`{|}~-@example.test", "LEGAL_EFFECTIVE_DATE" => "x" * 41 }
}
pages = states.map do |state, env|
  keys.each { |key| ENV.delete(key) }
  env.each { |key, value| ENV[key] = value }
  bodies = %w[about privacy terms].to_h do |page|
    session = ActionDispatch::Integration::Session.new(Rails.application)
    session.host! "campfire.test"
    session.get "/#{page}", headers: { "Accept" => "text/html" }
    ActiveSupport::IsolatedExecutionState.clear
    raise "public page failed: #{state}/#{page}" unless session.response.status == 200
    raise "public page wrote a cookie" if session.response.headers["Set-Cookie"]
    [page, session.response.body]
  end
  { state: state, env: env, policy: { operator_name: PublicPolicy.operator_name, contact_email: PublicPolicy.contact_email, effective_date: PublicPolicy.effective_date }, bodies: bodies }
end
keys.each { |key| ENV.delete(key) }
qr = ["http://example.com", "http://campfire.test", "こんにちは", "x" * 3000].map do |value|
  id = Base64.urlsafe_encode64(value)
  session = ActionDispatch::Integration::Session.new(Rails.application)
  session.host! "campfire.test"
  session.get "/qr_code/#{id.empty? ? '=' : id}"
  ActiveSupport::IsolatedExecutionState.clear
  { input: value, id: id, status: session.response.status, body: session.response.status == 200 ? session.response.body : nil, cache_control: session.response.headers["Cache-Control"] }
end
policy_inputs = [nil, "", "\t \r\n", "\u00a0Name\u00a0", "\u2003Name\u2003", "Jan 2, 2027", "日本 2027", "𝒜 2027", "x" * 40, "x" * 41, "a\nb", "a\tb"]
policy_inputs += ["  Acme Widgets  ", " privacy@example.test ", "not-an-email", "a@b", "@example.test", "a b@example.test",
  "privacy@example.test\nBcc: evil@example.test", "privacy@example.test\r\nSubject: hi", '"><script>alert(1)</script>',
  "privacy@example.test,other@example.test", "privacy@example.test;other@example.test", "<privacy@example.test>",
  "January 2, 2027", "  ", "<script>alert(1)</script>", "\u00a0", "² 2027", "Ⅰ 2027", "A\u0301 2027"]
policy = policy_inputs.map do |value|
  keys.each { |key| value.nil? ? ENV.delete(key) : ENV[key] = value }
  { input: value, operator_name: PublicPolicy.operator_name, contact_email: PublicPolicy.contact_email, effective_date: PublicPolicy.effective_date }
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), pages: pages, policy: policy, qr: qr)
warn "Rails public oracle: #{pages.length * 3} page bodies, #{policy.length} policy inputs, #{qr.length} QR cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
