# Actual thread-content requests plus fixed-secret view renders (the core-view oracle contract).
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
ApplicationController.allow_forgery_protection = false
class GoldenThreadController < ChannelThreadsController
  def self.controller_name = 'channel_threads'
  def protect_against_forgery? = true
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
room = Room.find(486777696)
viewer = User.find(127326141)
creator = User.find(149087659)
parent = room.root_messages.create!(creator:, markdown_source: 'Pane parent', client_message_id: 'pane-parent')
thread = ChannelThread.create!(room:, creator:, name: 'Pane <&> thread', parent_message: parent)
other = ChannelThread.create!(room:, creator:, name: 'Other pane')
foreign = other.messages.create!(room:, creator:, markdown_source: 'Foreign reply', client_message_id: 'pane-foreign')
messages = 45.times.map { |i| thread.messages.create!(room:, creator:, markdown_source: "Pane reply #{i}", client_message_id: "pane-#{i}") }
# Read-only presentation fixture; the agent mutation API belongs to WS11.
AgentStep.insert_all!([{channel_thread_id: thread.id, agent_id: 0, name: 'Thread action', status: 'done', position: 0, input_summary: 'Input <&>', output_summary: 'Output <&>', duration_ms: 1500, created_at: Time.current, updated_at: Time.current}])
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed[:session_token] = viewer.sessions.where.not(two_factor_verified_at: nil).first!.token
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = { 'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
rows = []
base = "/rooms/#{room.id}/threads/#{thread.id}/content"
[ ['latest', base, {}], ['first', base, {message_id: messages.first.id}], ['middle', base, {message_id: messages[22].id}],
  ['last', base, {message_id: messages.last.id}], ['blank', base, {message_id: ''}], ['cross_root', base, {message_id: parent.id}],
  ['cross_thread', base, {message_id: foreign.id}], ['wrong_room', "/rooms/699448326/threads/#{thread.id}/content", {}],
  ['json', "#{base}.json", {}] ].each do |name, path, params|
  browser.get(path, params:, headers: headers.dup)
  response = browser.response
  selected = browser.controller.view_assigns['messages'] if response.successful?
  anchor = browser.controller.view_assigns['content_anchor'] if response.successful?
  Current.user = viewer
  composer_button = GoldenThreadController.renderer.new(http_host: 'campfire.test', https: false).render(partial: 'scheduled_messages/composer_button', locals: {room:, thread:})
  body = if response.successful?
    GoldenThreadController.renderer.new(http_host: 'campfire.test', https: false).render(partial: 'channel_threads/conversation', locals: {room:, thread:, messages: selected, anchor_message_id: anchor&.id})
  end
  rows << {name:, path:, params:, status: response.status, content_type: response.headers['Content-Type'], cache_control: response.headers['Cache-Control'],
    at_latest: response.headers['X-Thread-Content-At-Latest'], anchor: anchor&.id, selected_ids: selected&.map(&:id), body:, composer_button:}
end
Current.user = viewer
room_composer = GoldenThreadController.renderer.new(http_host: 'campfire.test', https: false).render(partial: 'rooms/show/composer', locals: {room:, inline: true})
room_schedule = GoldenThreadController.renderer.new(http_host: 'campfire.test', https: false).render(partial: 'scheduled_messages/composer_button', locals: {room:})
room_footer = GoldenThreadController.renderer.new(http_host: 'campfire.test', https: false).render(inline: '<% render "rooms/show/composer", room: @room %><%= content_for :footer %>', layout: false, assigns: {room:})
pending_template = GoldenThreadController.renderer.new(http_host: 'campfire.test', https: false).render(partial: 'messages/template')
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], parent_id: parent.id, thread_id: thread.id, other_id: other.id, foreign_id: foreign.id,
  message_ids: messages.map(&:id), room_updated_at: room.updated_at.iso8601(3), slash_names: SlashCommands::Registry.all.map(&:name), rows:, room_composer:, room_schedule:, room_footer:, pending_template:) + "\n")
puts "WS8bm thread-content oracle: #{rows.size} actual requests; anchor scope and fixed-secret conversation/composer bytes"
