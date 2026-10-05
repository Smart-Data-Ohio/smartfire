# Fresh, real-controller receipts for branches absent from the existing byte fixtures.
# parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze \
#   reference-tools/template_coverage/http.rb vectors/template_coverage_http.json
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.application.config.hosts.clear
ActionController::Base.allow_forgery_protection = true
module CoverageRenderSecrets
  # Request verification is irrelevant to rendering receipts; keep real form fields.
  def verify_authenticity_token
  end

  def protect_against_forgery?
    true
  end
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
ApplicationController.prepend(CoverageRenderSecrets)
Rails.application.config.content_security_policy_nonce_generator = ->(_) { 'NONCE' }

user = User.find(127326141)
session = user.sessions.where.not(two_factor_verified_at: nil).first!
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = session.token
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = { 'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
traces = []
subscriber = ActiveSupport::Notifications.subscribe(/render_(?:template|partial|collection)\.action_view/) do |event|
  path = event.payload[:identifier]
  traces << path.delete_prefix("#{Rails.root}/app/views/") if path&.start_with?("#{Rails.root}/app/views/")
end
cases = []
get = ->(name, path, region = "main") do
  traces.clear
  browser.get(path, headers: headers.merge(region == 'stream' ? { 'Accept' => 'text/vnd.turbo-stream.html' } : {}))
  response = browser.response
  expected_status = name == "lexxy_users_not_reachable" ? 406 : 200
  raise "#{name}: #{response.status}" unless response.status == expected_status
  html = response.body
  # Compare the entire main-content region: the shared layout has separate existing
  # receipts, and assets in its head deliberately track post-pin fixes on main.
  html = html.split('<main id="main-content">', 2).fetch(1).split('</main>', 2).first if region == 'main' && response.status == 200
  cases << { name: name, path: path, region: region, status: response.status, body: html, templates: traces.uniq.sort }
  ActiveSupport::IsolatedExecutionState.clear
end
get.call('board_new', '/rooms/boards/new')
get.call('board_edit', '/rooms/boards/699448332/edit')
get.call('user_self', '/users/127326141')
get.call('user_peer', '/users/712064548')
[[2, 'banned'], [1, 'deactivated'], [0, 'active_again']].each do |status, name|
  User.find(712064548).update_columns(status: status)
  get.call("user_#{name}", '/users/712064548')
  cases.last[:setup] = { user_status: status }
end
%w[everything mentions nothing].each do |level|
  Membership.find_by!(user_id: user.id, room_id: 486777696).update_columns(involvement: Membership.involvements.fetch(level))
  get.call("involvement_#{level}", '/rooms/486777696/involvement')
end
get.call('lexxy_users_not_reachable', '/autocompletable/users?query=David', 'full')
# The rescue partial must also be exercised, not credited from an ordinary message.
message = Message.where(room_id: 486777696, thread_id: nil).order(:id).first!
ActiveRecord::Base.connection.disable_referential_integrity do
  message.update_columns(creator_id: 2_100_000_000)
end
get.call('missing_creator', "/rooms/486777696/messages/#{message.id}")
cases.last[:setup] = { missing_creator_message: message.id }
# Cross the fixed 500-record page boundary so both the deferred next-page frame
# and its last-page removal are compared, not just an empty collection.
rows = 501.times.map { |n| { id: 2_000_000_000 + n, name: format('Coverage member %03d', n), role: 0, status: 0, created_at: Time.current, updated_at: Time.current } }
User.insert_all!(rows)
get.call('users_page_one', '/account/users?page=1', 'stream')
get.call('users_page_two', '/account/users?page=2', 'stream')
# Both Turbo create declarations need an HTTP byte receipt; JSON writes do not cover them.
thread_id = 1_900_500_000
ChannelThread.insert_all!([{ id: thread_id, room_id: 486777696, creator_id: user.id, name: 'Coverage thread', last_activity_at: Time.current, created_at: Time.current, updated_at: Time.current }])
[['root_create', '/rooms/486777696/messages'], ['thread_create', "/rooms/486777696/threads/#{thread_id}/messages"]].each do |name, path|
  traces.clear
  input = { markdown_source: 'Coverage **message**', client_message_id: "coverage-#{name}" }
  browser.post(path, params: { message: input }, headers: headers.merge('Accept' => 'text/vnd.turbo-stream.html'))
  response = browser.response
  raise "#{name}: #{response.status}" unless response.status == 200
  cases << { name: name, method: 'POST', path: path, input: input, region: 'stream', status: response.status, body: response.body, templates: traces.uniq.sort }
  ActiveSupport::IsolatedExecutionState.clear
end
traces.clear
browser.post('/rooms/999999999/messages', params: { message: { markdown_source: 'Gone room' } }, headers: headers)
raise 'room_not_found did not render' unless browser.response.status == 200
cases << { name: 'room_not_found', method: 'POST', path: '/rooms/999999999/messages', input: { markdown_source: 'Gone room' }, region: 'main', status: browser.response.status, body: browser.response.body.split('<main id="main-content">', 2).fetch(1).split('</main>', 2).first, templates: traces.uniq.sort }
# Existing room goldens all exceed the 40-message paging boundary. Exercise the
# original-room invitation with an empty timeline, through the same controller.
original = Room.original
ActiveRecord::Base.connection.disable_referential_integrity do
  Message.where(room_id: original.id).delete_all
end
get.call('original_room_invitation', "/rooms/#{original.id}")
cases.last[:setup] = { empty_original_room: original.id }
traces.clear
input = { markdown_source: 'Coverage invitation **message**', client_message_id: 'coverage-original-create' }
path = "/rooms/#{original.id}/messages"
browser.post(path, params: { message: input }, headers: headers.merge('Accept' => 'text/vnd.turbo-stream.html'))
raise 'original_create failed' unless browser.response.status == 200
cases << { name: 'original_create', method: 'POST', path: path, input: input, region: 'stream', status: browser.response.status, body: browser.response.body, templates: traces.uniq.sort }
ActiveSupport::IsolatedExecutionState.clear
get.call('original_room_invitation_populated', "/rooms/#{original.id}")
ActiveSupport::Notifications.unsubscribe(subscriber)
sources = cases.flat_map { |c| c[:templates] }.uniq.to_h { |path| ["app/views/#{path}", Digest::SHA256.file(Rails.root.join('app/views', path)).hexdigest] }
output = { reference: File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/reference.sha')).strip, now: Time.current.iso8601, inserted_users: rows.map { |row| row.except(:created_at, :updated_at) }, cases: cases, source_hashes: sources }
File.write(File.join(ENV.fetch('PARITY_WORK'), ARGV.fetch(0)), JSON.pretty_generate(output) + "\n")
warn "Template coverage Rails oracle: #{cases.size} real HTTP responses, #{sources.size} rendered templates"
