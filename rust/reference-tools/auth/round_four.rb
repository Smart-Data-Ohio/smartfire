# Pinned Rails is the oracle for identifier validation and account audit snapshots.
require "json"
require "nokogiri"
require "base64"
Rails.cache = ActiveSupport::Cache::MemoryStore.new
CHROME = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
def browser
  user = User.find(127326141)
  client = ActionDispatch::Integration::Session.new(Rails.application)
  client.host! "campfire.test"
  row = user.sessions.start!(user_agent: CHROME, ip_address: "127.0.0.1", two_factor_verified: true)
  request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
  request.cookie_jar.signed[:session_token] = row.token
  client.cookies[:session_token] = request.cookie_jar[:session_token]
  client.get "/users/me/profile", headers: { "User-Agent" => CHROME }
  raise "initial page: #{client.response.status}" unless client.response.status == 200
  token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')["content"]
  headers = { "User-Agent" => CHROME, "X-CSRF-Token" => token, "Accept" => "text/html" }
  client.post "/sudo", params: { password: "secret123456" }, headers: headers
  raise "sudo: #{client.response.status}" unless client.response.status == 302
  [client, headers]
end
def audits
  AuditLog.order(:id).map { |row| row.attributes.slice("action", "actor_id", "actor_label", "target_type", "target_id", "target_label", "details", "ip_address", "user_agent") }
end
def isolated
  result = nil
  User.transaction do
    [SessionsController, SudosController].each { |controller| controller.cache_store.clear }
    result = yield
    raise ActiveRecord::Rollback
  end
  result
end
identifiers = TZInfo::Timezone.all_identifiers.sort
mapping = ActiveSupport::TimeZone::MAPPING.sort.to_h
names = (identifiers + mapping.keys).uniq.sort
probes = (names.flat_map { |name| [name, name.downcase, name.upcase, " #{name}", "#{name} "] } + ["Mars/Olympus", "localtime", "posix/America/New_York", "right/America/New_York", "America/New_York\0", "America/New_York\n"]).uniq.sort.to_h { |name| [name, ActiveSupport::TimeZone[name].present?] }
File.write(ENV.fetch("WS9_TIME_ZONE_VECTORS"), JSON.pretty_generate({ identifiers:, mapping:, probes: }) + "\n")
zones = %w[america/new_york AMERICA/NEW_YORK America/new_York utc UTC America/New_York US/Eastern GMT Etc/GMT+5 Factory Mars/Olympus] + ["Eastern Time (US & Canada)", "eastern time (us & canada)", " America/New_York", "America/New_York ", "", " \t"]
profile = zones.to_h do |zone|
  [zone, isolated do
    user = User.find(127326141)
    user.update_columns(time_zone: "UTC", time_zone_explicit: false)
    client, headers = browser
    AuditLog.delete_all
    client.patch "/users/me/profile", params: { user: { time_zone: zone } }, as: :json, headers: headers
    { status: client.response.status, location: client.response.headers["Location"], zone: user.reload.time_zone,
      explicit: user.time_zone_explicit, audits: audits }
  end]
end
account = {}
logo_bytes = File.binread(Rails.root.join("app/assets/images/logos/app-icon.png"))
[
  ["settings", "patch", "/account", nil, { name: "Reviewer name", settings: { restrict_room_creation_to_administrators: "1" } }],
  ["settings_noop", "patch", "/account", nil, { name: "37signals", settings: { restrict_room_creation_to_administrators: false } }],
  ["settings_name", "patch", "/account", nil, { name: "New name" }],
  ["settings_bool", "patch", "/account", nil, { settings: { restrict_room_creation_to_administrators: true } }],
  ["settings_array_bool", "patch", "/account", nil, { settings: { restrict_room_creation_to_administrators: [] } }],
  ["settings_object_bool", "patch", "/account", nil, { settings: { restrict_room_creation_to_administrators: {} } }],
  ["settings_clear_logo", "patch", "/account", nil, { logo: "" }],
  ["settings_add_logo", "patch", "/account", nil, {}],
  ["settings_replace_logo", "patch", "/account", nil, {}],
  ["join_reset", "post", "/account/join_code", nil, {}],
  ["styles", "patch", "/account/custom_styles", nil, { custom_styles: "body { --review: 1 }" }],
  ["styles_unicode", "patch", "/account/custom_styles", nil, { custom_styles: "body { --review: '🔥' }" }],
  ["styles_empty", "patch", "/account/custom_styles", nil, { custom_styles: "" }],
  ["styles_noop", "patch", "/account/custom_styles", "body { --review: 1 }", { custom_styles: "body { --review: 1 }" }],
  ["styles_clear", "patch", "/account/custom_styles", "body { --review: 1 }", { custom_styles: nil }],
  ["styles_nil", "patch", "/account/custom_styles", nil, { custom_styles: nil }],
  ["styles_boolean", "patch", "/account/custom_styles", nil, { custom_styles: false }],
  ["styles_array", "patch", "/account/custom_styles", nil, { custom_styles: ["ignored"] }],
  ["logo_destroy", "delete", "/account/logo", nil, {}],
  ["logo_absent", "delete", "/account/logo", nil, {}]
].each do |name, method, path, initial_styles, params|
  account[name] = isolated do
    record = Account.first
    record.update_columns(name: "37signals", custom_styles: initial_styles, settings: { restrict_room_creation_to_administrators: false })
    initial_logo = !%w[logo_absent settings_add_logo].include?(name)
    if initial_logo
      record.logo.attach(io: StringIO.new(logo_bytes), filename: "logo.png", content_type: "image/png")
    else
      record.logo.detach
    end
    client, headers = browser
    AuditLog.delete_all
    before_code = record.join_code
    upload = name.start_with?("settings_add_logo", "settings_replace_logo")
    if upload
      file = Tempfile.new(["ws9-logo", ".png"])
      file.binmode
      file.write(logo_bytes)
      file.rewind
      uploaded = Rack::Test::UploadedFile.new(file.path, "image/png", true, original_filename: "logo.png")
      client.public_send(method, path, params: { account: { logo: uploaded } }, headers: headers)
      file.close!
    else
      client.public_send(method, path, params: { account: params }, as: :json, headers: headers)
    end
    record.reload
    { method:, path:, params:, initial_styles:, initial_logo:, upload:, status: client.response.status,
      location: client.response.headers["Location"], name: record.name, styles: record.custom_styles,
      restrict: record.settings.restrict_room_creation_to_administrators?, logo: record.logo.attached?,
      code_changed: before_code != record.join_code, audits: audits }
  end
end
File.write(ENV.fetch("WS9_ROUND_FOUR_VECTORS"), JSON.pretty_generate({ profile:, account:, logo_bytes: Base64.strict_encode64(logo_bytes) }) + "\n")
puts "WS9 round-four Rails vectors: #{identifiers.size} exact identifiers, #{mapping.size} aliases, #{probes.size} lookup probes, #{profile.size} profile requests, #{account.size} account requests"
