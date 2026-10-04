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
session.get "/account/edit"
ActiveSupport::IsolatedExecutionState.clear
token = Nokogiri::HTML(session.response.body).at_css('meta[name="csrf-token"]')["content"]
cases = [
  ["rails_name", {}, "Pacific Time (US & Canada)"], ["iana", {}, "America/New_York"],
  ["saved_wins", { time_zone: "Eastern Time (US & Canada)" }, "America/Los_Angeles"],
  ["explicit_blank", { time_zone_explicit: true }, "America/New_York"],
  ["unknown", {}, "Narnia"], ["wrong_case", {}, "america/new_york"], ["padded", {}, " America/New_York "],
  ["nil", {}, nil], ["array", {}, ["America/New_York"]], ["hash", {}, { time_zone: "America/New_York" }], ["boolean", {}, false],
  ["invalid_user", { theme: "unknown" }, "America/New_York"],
  ["invalid_voice", { voice_mode: "unknown" }, "America/New_York"],
  ["incomplete_quiet", { quiet_hours_enabled: true }, "America/New_York"],
  ["invalid_user_unknown_zone", { theme: "unknown" }, "Narnia"],
  ["blank_saved", { time_zone: " " }, "America/New_York"],
  ["path_user_id_ignored", {}, "America/New_York"]
].map do |name, initial, zone|
  user.update_columns(time_zone: nil, time_zone_explicit: false, theme: "system", voice_mode: nil, quiet_hours_enabled: false,
    quiet_hours_start_minute: nil, quiet_hours_end_minute: nil, updated_at: 1.hour.ago, **initial)
  path = name == "path_user_id_ignored" ? "/users/149087659/time_zone" : "/users/me/time_zone"
  session.patch path, params: { time_zone: zone, user_id: 149087659 }, as: :json, headers: { "X-CSRF-Token" => token }
  ActiveSupport::IsolatedExecutionState.clear
  { name: name, path: path, initial: initial, zone: zone, status: session.response.status, json: session.response.parsed_body,
    state: user.reload.attributes.slice("time_zone", "time_zone_explicit").merge("updated_at" => user.updated_at.iso8601(6)) }
end
user.update_columns(theme: "system", voice_mode: nil, quiet_hours_enabled: false, tour_completed_at: nil, updated_at: 1.hour.ago)
session.patch "/users/149087659/tour", headers: { "X-CSRF-Token" => token }
ActiveSupport::IsolatedExecutionState.clear
tour = { status: session.response.status, completed_at: user.reload.tour_completed_at.iso8601(6), updated_at: user.updated_at.iso8601(6) }
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), time_zones: cases, tour: tour)
warn "Rails preference oracle: #{cases.length} time-zone cases, 1 tour touch; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
