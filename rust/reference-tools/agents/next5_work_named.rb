# Reuse the committed oracle's fixture/state extractor. Only harness inputs and
# request sequencing change; Rails producers and callbacks remain unchanged.
require "json"
source = File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/agents/work_writes_http_contract.rb"))
def replace_once(source, before, after)
  raise "oracle harness drift: #{before}" unless source.scan(Regexp.new(Regexp.escape(before))).size == 1
  source.sub(before, after)
end
source = replace_once(source, "conn=ActiveRecord::Base.connection", <<'INPUTS'.chomp)
cases = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/agents/next5-work-named-inputs.json")), symbolize_names: true).fetch(:cases).map do |row|
  last = row.fetch(:steps).last
  row.merge(name: row.fetch(:key), surface: "named", method: last.fetch(:method), path: last.fetch(:path), body: last[:body], setup: {grant: grants, board: true, work_write: true}.merge(row.fetch(:setup)))
end
conn=ActiveRecord::Base.connection
INPUTS
source = replace_once(source, <<'BEFORE'.chomp, <<'AFTER'.chomp)
setup[:grant].each{|cap|AgentGrant.create!(agent:agent,capability:cap,room_id:setup[:grant_room],granted_by_id:127326141,revoked_at:setup[:revoked] ? Time.current : nil)}
BEFORE
setup[:grant].each do |cap|
  scope = cap == "manage_threads" ? setup.fetch(:manage_room, setup[:grant_room]) : cap == "read_messages" ? setup.fetch(:read_room, setup[:grant_room]) : setup[:grant_room]
  AgentGrant.create!(agent:agent,capability:cap,room_id:scope,granted_by_id:127326141,revoked_at:setup[:revoked] ? Time.current : nil)
end
AFTER
source = replace_once(source, <<'BEFORE'.chomp, <<'AFTER'.chomp)
ActiveSupport::Notifications.subscribed(callback,"sql.active_record") {session.public_send(item[:method],item[:path],params:body,headers:headers)}
BEFORE
observations = []
item.fetch(:steps).each do |step|
  session = ActionDispatch::Integration::Session.new(Rails.application)
  session.host! "campfire.test"
  request_headers = headers.dup
  request_path = step.fetch(:path)
  case step[:auth]
  when "session"
    request_headers.delete("Authorization")
    cookie = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "vectors/campfire_sessions.json"))).fetch("sessions").find { |s| s.fetch("user_name") == "David" }.fetch("cookie_header")
    cookie.split("; ").each { |part| key, value = part.split("=", 2); session.cookies[key] = CGI.unescape(value) }
    session.get("/rooms/#{room}", headers: {"Accept" => "text/html"})
    token = session.response.body[/name="csrf-token" content="([^"]+)"/, 1]
    raise "session fixture must receive a real CSRF token" unless token
    request_headers["X-CSRF-Token"] = CGI.unescapeHTML(token)
  when "bot_key"
    request_headers.delete("Authorization")
    request_path += "?bot_key=394959859-BenderToken1"
  end
  request_body = step[:body]&.to_json
  ActiveSupport::Notifications.subscribed(callback,"sql.active_record") {session.public_send(step.fetch(:method),request_path,params:request_body,headers:request_headers)}
  response = session.response
  observations << step.merge(request_body: request_body, status: response.status, response_body: response.body, response_headers: %w[Content-Type Cache-Control Pragma Retry-After Location].to_h { |name| [name, response.headers[name]] })
end
item[:observations] = observations
AFTER
eval(source, TOPLEVEL_BINDING, "next5 named committed oracle")
