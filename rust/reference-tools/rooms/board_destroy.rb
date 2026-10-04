# The unchanged board DELETE route, including its inherited nil-room failure.
require "json"
require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
Rails.logger = Logger.new($stderr, level: Logger::WARN)
{
  "app/controllers/rooms/boards_controller.rb" => "2b0873b3f1524c3f484d1913169d2813d29af70a20e41d4e3e71a7e3addf51ad",
  "app/controllers/rooms_controller.rb" => "53c7fd8b0619e478425d492f1be3abbc11affafa1b992dea09f8cd0b413c94eb"
}.each { |file, hash| raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
ActiveJob::Base.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16)

def browser(user_id)
  user = User.find(user_id)
  request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
  request.cookie_jar.signed.permanent[:session_token] = { value: user.sessions.first.token, httponly: true, same_site: :lax }
  client = ActionDispatch::Integration::Session.new(Rails.application)
  client.host! "campfire.test"
  client.cookies["session_token"] = request.cookie_jar[:session_token]
  client.get "/account/edit"
  token = Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')&.[]("content")
  raise "no CSRF token" unless token
  ActiveSupport::IsolatedExecutionState.clear
  [client, token]
end

# Snapshot full rows, including queue records, before the request. Session activity is
# intentionally outside this domain snapshot; no room destruction work may begin.
UNCHANGED_TABLES = %w[rooms memberships messages channel_threads audit_logs background_jobs
  active_storage_attachments active_storage_blobs active_storage_variant_records activity_items
  board_tag_assignments board_sla_rules board_sla_nudges board_stale_digests boosts]
def domain_snapshot
  connection = ActiveRecord::Base.connection
  UNCHANGED_TABLES.to_h { |table| [table, connection.select_all("SELECT * FROM #{connection.quote_table_name(table)} ORDER BY id").to_a] }
end

cases = []
[
  ["admin_json", 127326141, 127326141, [127326141, 773523953], "json"],
  ["creator_html", 773523953, 773523953, [127326141, 773523953], "html"],
  ["admin_turbo", 127326141, 773523953, [127326141, 773523953], "turbo_stream"],
  ["forbidden_member", 773523953, 127326141, [127326141, 773523953], "json"],
  ["inaccessible_admin", 127326141, 773523953, [773523953], "json"],
  ["missing", 127326141, 127326141, [127326141], "json"],
  ["wrong_type", 127326141, 127326141, [127326141], "json"]
].each_with_index do |(name, actor, creator, members, format), index|
  id = 2117000000 + index
  room = Room.create!(id: id, type: name == "wrong_type" ? "Rooms::Closed" : "Rooms::Board", name: "Launch", creator_id: creator)
  members.each { |user_id| room.memberships.create!(user_id: user_id) }
  client, token = browser(actor)
  path = "/rooms/boards/#{name == "missing" ? 2117999999 : id}"
  path += ".json" if format == "json"
  broadcasts = []
  headers = { "X-CSRF-Token" => token, "Accept" => {"html" => "text/html", "json" => "application/json", "turbo_stream" => "text/vnd.turbo-stream.html, text/html"}.fetch(format) }
  subscription = ActiveSupport::Notifications.subscribe("broadcast.action_cable") { |*args| broadcasts << args.last.slice(:broadcasting, :message) }
  jobs_before = ActiveJob::Base.queue_adapter.enqueued_jobs.size
  before = domain_snapshot
  client.delete path, headers: headers
  ActiveSupport::Notifications.unsubscribe(subscription)
  raise "domain write during #{name}" unless before == domain_snapshot
  raise "unexpected response during #{name}" unless client.response.status == 500
  error = client.request.env.fetch("action_dispatch.exception")
  raise "unexpected exception during #{name}: #{error.inspect}" unless error.is_a?(NoMethodError) && error.name == :name
  ActiveSupport::IsolatedExecutionState.clear
  response = {status: client.response.status, body: client.response.body, headers: client.response.headers.slice("Content-Type", "Location", "Cache-Control")}
  flash = client.request.flash.to_hash
  room.reload
  audit = AuditLog.where(action: "room.destroy", target_id: id).last
  state = {deleted: room.deleted?, claimed: room.destroy_enqueued_at.present?, memberships: room.memberships.count,
    audit: audit&.attributes&.slice("action", "actor_id", "target_type", "target_id", "target_label", "details")}
  jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.drop(jobs_before).map { |job| [job[:job].name, job[:args]] }
  raise "unexpected jobs or broadcasts during #{name}" unless jobs.empty? && broadcasts.empty?
  cases << {name: name, id: id, actor: actor, creator: creator, members: members, format: format, path: path, response: response, flash: flash, state: state, broadcasts: broadcasts, jobs: jobs, unchanged_tables: UNCHANGED_TABLES, exception: {class: error.class.name, method: error.name, source: error.backtrace.find { |line| line.include?("app/controllers/rooms_controller.rb:") }&.sub(Rails.root.to_s + "/", "")}}
end
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA"), notes: "The unchanged board DELETE route raises NoMethodError on nil @room.name at rooms_controller.rb:29 because the subclass callback scopes exclude destroy. The lead decision is to reproduce this Rails reference error, as with directs#show. All seven responses are from /rooms/boards/:id itself; complete domain rows, durable queue, enqueued jobs and broadcasts remain unchanged. Board deletion by the UI continues through the existing /rooms/:id route. Nothing remains flagged.", cases: cases)
warn "Rails board DELETE: #{cases.size} unchanged-route HTTP/state/broadcast comparisons; 7 reference 500s; zero domain writes, jobs or broadcasts"
