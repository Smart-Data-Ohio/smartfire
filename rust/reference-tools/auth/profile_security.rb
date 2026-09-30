# Observable HTTP vectors from pinned Rails, with real signed cookies and CSRF.
require "json"
require "nokogiri"
Rails.cache = ActiveSupport::Cache::MemoryStore.new
CHROME = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
def browser(user = nil)
  client = ActionDispatch::Integration::Session.new(Rails.application)
  client.host! "campfire.test"
  if user
    row = user.sessions.start!(user_agent: CHROME, ip_address: "127.0.0.1", two_factor_verified: true)
    request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
    request.cookie_jar.signed[:session_token] = row.token
    client.cookies[:session_token] = request.cookie_jar[:session_token]
  end
  client.get(user ? "/users/me/profile" : "/session/new", headers: { "User-Agent" => CHROME })
  raise "initial page: #{client.response.status}" unless client.response.status == 200
  token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')["content"]
  [client, { "User-Agent" => CHROME, "X-CSRF-Token" => token }]
end
def audits
  AuditLog.order(:id).map { |row| row.attributes.slice("action", "actor_id", "actor_label", "target_type", "target_id", "target_label", "details", "ip_address", "user_agent") }
end
cases = [
  ["missing", { email_address: "ws9-reviewed@example.test", password: "proposed-password", name: "Submitted name", bio: "Submitted bio" }],
  ["blank", { email_address: "ws9-reviewed@example.test", current_password: "  " }],
  ["wrong", { email_address: "ws9-reviewed@example.test", current_password: "wrong" }],
  ["new_is_not_current", { email_address: "ws9-reviewed@example.test", password: "proposed-password", current_password: "proposed-password" }],
  ["correct", { email_address: "ws9-reviewed@example.test", current_password: "secret123456" }],
  ["email_and_password", { email_address: "ws9-reviewed@example.test", password: "proposed-password", current_password: "secret123456" }],
  ["case_only", { email_address: "DAVID@37SIGNALS.COM", name: "Submitted name", bio: "Submitted bio" }],
  ["whitespace_only", { email_address: "  david@37signals.com  " }],
  ["password_only", { password: "proposed-password" }],
  ["passwordless", { email_address: "ws9-reviewed@example.test" }],
  ["unicode_case_only", { email_address: "straße@example.test" }],
  ["nil_email_correct", { email_address: nil, name: nil, bio: nil, current_password: "secret123456" }],
  ["nil_email_missing", { email_address: nil, password: "proposed-password" }],
  ["array_email_missing", { email_address: ["ws9-reviewed@example.test"], name: "Submitted name" }],
  ["hash_email_missing", { email_address: { value: "ws9-reviewed@example.test" }, name: "Submitted name" }]
]
profile = cases.to_h do |name, params|
  result = nil
  User.transaction do
    AuditLog.delete_all
    user = User.find(127326141)
    user.update_columns(password_digest: nil) if name == "passwordless"
    user.update_columns(email_address: "STRASSE@example.test") if name == "unicode_case_only"
    before = user.attributes.slice("email_address", "name", "bio", "password_digest", "email_self_changed_at", "google_email_link_allowed")
    client, headers = browser(user)
    if params.values.any? { |value| !value.is_a?(String) }
      client.patch "/users/me/profile", params: { user: params }, as: :json, headers: headers.merge("Accept" => "text/html")
    else
      client.patch "/users/me/profile", params: { user: params }, headers: headers
    end
    body = client.response.body
    user.reload
    result = {
      params:, status: client.response.status, email: user.email_address, name: user.name, bio: user.bio,
      password_changed: before["password_digest"] != user.password_digest,
      marker: user.email_self_changed_at&.iso8601(6), allowed: user.google_email_link_allowed,
      audits: audits,
      current_password_input: body[/<input[^>]+id="user_current_password"[^>]*>/],
      error_html: body[/        <p class="txt-small margin-none" style="color: var\(--color-negative\)">\n          Current password .*?\n        <\/p>\n/m]
    }
    raise ActiveRecord::Rollback
  end
  [name, result]
end
failure = []
User.transaction do
  AuditLog.delete_all
  Rails.cache.clear
  client, headers = browser
  emails = Array.new(10, "david@37signals.com") + ["david@37signals.com", "another@example.test", "a-password-in-email", "ANOTHER@example.test"] + (1..21).map { |i| "spray#{i}@example.test" }
  emails.each do |email|
    client.post "/session", params: { email_address: email, password: "wrong" }, headers: headers
    failure << { email:, status: client.response.status, audits: audits }
  end
  raise ActiveRecord::Rollback
end
transfer = nil
User.transaction do
  AuditLog.delete_all
  client, headers = browser
  client.put "/session/transfers/invalid-transfer", headers: headers
  transfer = { status: client.response.status, audits: audits }
  raise ActiveRecord::Rollback
end
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
failure_windows = []
User.transaction do
  AuditLog.delete_all
  Rails.cache.clear
  client, headers = browser
  start = Time.current
  [[0, "192.0.2.10", "david@37signals.com"], [0, "192.0.2.20", "DAVID@37SIGNALS.COM"], [299, "192.0.2.10", "DAVID@37SIGNALS.COM"], [300, "192.0.2.10", "david@37signals.com"], [301, "192.0.2.10", "david@37signals.com"]].each do |offset, ip, email|
    travel_to start + offset
    client.post "/session", params: { email_address: email, password: "wrong" }, headers: headers.merge("X-Forwarded-For" => ip)
    failure_windows << { offset:, ip:, email:, status: client.response.status, audits: audits }
  end
  travel_back
  raise ActiveRecord::Rollback
end
File.write(ENV.fetch("WS9_PROFILE_SECURITY_VECTORS"), JSON.pretty_generate({ profile:, failure:, transfer:, failure_windows: }) + "\n")
puts "WS9 profile security Rails vectors: #{profile.size} profile cases, #{failure.size} failed sign-ins, 1 rejected transfer, #{failure_windows.size} failure-window/IP cases"
