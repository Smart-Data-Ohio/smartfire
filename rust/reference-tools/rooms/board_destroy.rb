# Inherited RoomsController#destroy, through the real board route and session/CSRF stack.
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
  client.delete path, headers: headers
  board_status = client.response.status
  # The pinned board subclass accidentally removes destroy from both inherited callbacks.
  # Preserve that finding; do not modify Rails or claim these are board-route success bytes.
  # Exercise the exact inherited destroy action via its working base route on the same board.
  reference_path = path.sub("/rooms/boards/", "/rooms/")
  subscription = ActiveSupport::Notifications.subscribe("broadcast.action_cable") { |*args| broadcasts << args.last.slice(:broadcasting, :message) }
  jobs_before = ActiveJob::Base.queue_adapter.enqueued_jobs.size
  client.delete reference_path, headers: headers unless name == "wrong_type"
  ActiveSupport::Notifications.unsubscribe(subscription)
  ActiveSupport::IsolatedExecutionState.clear
  response = {status: client.response.status, body: client.response.body, headers: client.response.headers.slice("Content-Type", "Location", "Cache-Control")}
  flash = client.request.flash.to_hash
  room.reload
  audit = AuditLog.where(action: "room.destroy", target_id: id).last
  state = {deleted: room.deleted?, claimed: room.destroy_enqueued_at.present?, memberships: room.memberships.count,
    audit: audit&.attributes&.slice("action", "actor_id", "target_type", "target_id", "target_label", "details")}
  jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.drop(jobs_before).map { |job| [job[:job].name, job[:args]] }
  cases << {name: name, id: id, actor: actor, creator: creator, members: members, format: format, path: path, reference_path: reference_path, board_route_status: board_status, response: response, flash: flash, state: state, broadcasts: broadcasts, jobs: jobs} unless name == "wrong_type"
end
puts JSON.pretty_generate(reference: "d7c7de92", notes: "Board DELETE itself raises NoMethodError at rooms_controller.rb:29 before writes: its subclass callback scopes omit destroy. These are unmodified inherited-action responses via /rooms/:id on boards; the board-route crash remains flagged pending a Rails fix/decision.", cases: cases)
warn "Rails inherited board destroy: #{cases.size} complete HTTP/state/broadcast comparisons; board-route crash retained explicitly"
