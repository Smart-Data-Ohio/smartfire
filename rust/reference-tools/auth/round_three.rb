# Review regressions from OUR pinned Rails: real sessions, sudo and CSRF.
require "json"
require "nokogiri"
Rails.cache = ActiveSupport::Cache::MemoryStore.new
CHROME = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
DAVID = 127326141
KEVIN = 712064548
def browser(user = nil, sudo: false)
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
  headers = { "User-Agent" => CHROME, "X-CSRF-Token" => token, "Accept" => "text/html" }
  if sudo
    client.post "/sudo", params: { password: "secret123456" }, headers: headers
    raise "sudo: #{client.response.status}" unless client.response.status == 302
  end
  [client, headers]
end
def audits
  AuditLog.order(:id).map { |row| row.attributes.slice("action", "actor_id", "actor_label", "target_type", "target_id", "target_label", "details", "ip_address", "user_agent") }
end
def isolated
  result = nil
  User.transaction do
    Rails.cache.clear
    [SessionsController, SudosController].each { |controller| controller.cache_store.clear }
    AuditLog.delete_all
    result = yield
    raise ActiveRecord::Rollback
  end
  result
end
password = {
  array: ["ignored"], hash: { value: "ignored" }, empty_array: [], empty_hash: {}, nil: nil
}.to_h do |name, value|
  [name, isolated do
    user = User.find(DAVID)
    user.two_factor_remembered_devices.delete_all
    TwoFactorRememberedDevice.create_for!(user, user_agent: CHROME, ip_address: "127.0.0.1")
    before = user.password_digest
    client, headers = browser(user)
    client.patch "/users/me/profile", params: { user: { password: value } }, as: :json, headers: headers
    { value:, status: client.response.status, changed: before != user.reload.password_digest,
      devices: user.two_factor_remembered_devices.count, audits: audits }
  end]
end
email = {
  array: ["david@37signals.com"], two: ["david@37signals.com", "kevin@37signals.com"],
  escaped: ["dav\"id@37signals.com"], empty_array: [], nil: nil
}.to_h do |name, value|
  [name, isolated do
    client, headers = browser
    client.post "/session", params: { email_address: value, password: "wrong" }, as: :json, headers: headers
    { value:, status: client.response.status, audits: audits }
  end]
end
rate = isolated do
  client, headers = browser
  10.times { client.post "/session", params: { email_address: "david@37signals.com", password: "wrong" }, headers: headers }
  value = ["david@37signals.com"]
  client.post "/session", params: { email_address: value, password: "wrong" }, as: :json, headers: headers
  { value:, status: client.response.status, audits: audits }
end
zone = {
  blank: { time_zone: "" }, selected: { time_zone: "America/New_York" }, nil: { time_zone: nil },
  whitespace: { time_zone: " \t" }, alias: { time_zone: "Eastern Time (US & Canada)" },
  array: { time_zone: ["ignored"] }, hash: { time_zone: { value: "ignored" } },
  absent: { bio: "Updated bio" },
  rejected: { time_zone: "", email_address: "changed@example.test" },
  invalid: { time_zone: "Mars/Olympus", email_address: "changed@example.test", current_password: "secret123456", password: "proposed-password", bio: "Submitted bio" },
  already_explicit: { time_zone: "America/New_York" }
}.to_h do |name, params|
  [name, isolated do
    user = User.find(DAVID)
    user.update_columns(time_zone: "UTC", time_zone_explicit: name == :already_explicit)
    client, headers = browser(user)
    client.patch "/users/me/profile", params: { user: params }, as: :json, headers: headers
    { params:, status: client.response.status, zone: user.reload.time_zone, explicit: user.time_zone_explicit, audits: audits }
  end]
end
admin = {}
[
  ["promote", "patch", "account", "active", "member", { user: { role: "administrator" } }],
  ["demote", "patch", "account", "active", "administrator", { user: { role: "member" } }],
  ["role_noop", "patch", "account", "active", "member", { user: { role: "member" } }],
  ["invalid_role", "patch", "account", "active", "administrator", { user: { role: ["administrator"] } }],
  ["deactivate", "delete", "account", "active", "member", {}],
  ["ban", "post", "ban", "active", "member", {}],
  ["ban_noop", "post", "ban", "banned", "member", {}],
  ["ban_deactivated", "post", "ban", "deactivated", "member", {}],
  ["unban", "delete", "ban", "banned", "member", {}],
  ["unban_noop", "delete", "ban", "active", "member", {}],
  ["unban_deactivated", "delete", "ban", "deactivated", "member", {}]
].each do |name, method, route, status, role, params|
  admin[name] = isolated do
    user = User.find(KEVIN)
    user.update_columns(status: status, role: role)
    user.sessions.update_all(ip_address: "203.0.113.9")
    client, headers = browser(User.find(DAVID), sudo: true)
    AuditLog.delete_all # The action snapshot excludes the separately tested sudo row.
    path = route == "account" ? "/account/users/#{KEVIN}" : "/users/#{KEVIN}/ban"
    client.public_send(method, path, params: params, as: :json, headers: headers)
    { method:, route:, before_status: status, before_role: role, params:, status: client.response.status,
      user_status: user.reload.status, user_role: user.role, audits: audits }
  end
end
File.write(ENV.fetch("WS9_ROUND_THREE_VECTORS"), JSON.pretty_generate({ password:, email:, rate:, zone:, admin: }) + "\n")
puts "WS9 round-three Rails vectors: #{password.size} password, #{email.size} email, 1 rate-limit, #{zone.size} time-zone, #{admin.size} admin cases"
