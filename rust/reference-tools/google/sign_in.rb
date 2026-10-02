require "json"
require "digest"

ENV["GOOGLE_CLIENT_ID"] = "test-client-id"
ENV["GOOGLE_CLIENT_SECRET"] = "FAKE-google-client-secret"
ENV["GOOGLE_SIGN_IN_DOMAINS"] = "smartdata.net, other.test"
raw_state = "0123456789abcdef" * 2
nonce = "fedcba9876543210" * 2
verifier = Base64.urlsafe_encode64((0...32).to_a.pack("C*"), padding: false)
challenge = Base64.urlsafe_encode64(Digest::SHA256.digest(verifier), padding: false)
signed_state = Rails.application.message_verifier("google_sign_in_state").generate(raw_state)
redirect_uri = "http://test.host/session/google/callback"
urls = %w[sign_in link reauth sudo].to_h do |purpose|
  extra = %w[reauth sudo].include?(purpose) ? { prompt: "login", max_age: 0 } : {}
  [ purpose, Google::SignIn.authorize_url(redirect_uri:, state: signed_state, nonce:, challenge:, **extra) ]
end
paths = [nil, "", "/rooms/1", "/rooms/1?page=2#fragment", "http://example.test/rooms/1?page=2#fragment",
  "https://example.test:123/rooms/1", "http://EXAMPLE.test/rooms/1", "https://evil.test/rooms/1",
  "//evil.test/rooms/1", "/\\evil.test", "javascript:alert(1)", "http://example.test.evil.test/",
  "/a b", "/a%20b", "http://example.test", "/foo#bar", "http://user:pass@example.test/rooms/1"]
puts JSON.pretty_generate({
  reference: "d7c7de9264c63015be398001d7a1094e7695a6db",
  secret_key_base: ENV.fetch("SECRET_KEY_BASE"),
  client_id: ENV.fetch("GOOGLE_CLIENT_ID"), redirect_uri:, raw_state:, signed_state:, nonce:, verifier:, challenge:,
  authorize: urls, return_paths: paths.map { |path| { input: path, output: Google::SignIn.safe_return_path(path, host: "example.test") } }
})
