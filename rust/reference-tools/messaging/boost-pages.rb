require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
class GoldenBoostPagesController < Messages::BoostsController
  def self.controller_path = 'messages/boosts'
  def protect_against_forgery? = true
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
room = Room.find(486777696)
david = User.find(127326141)
jason = User.find(149087659)
message = room.root_messages.create!(creator: david, markdown_source: 'Boost pages', client_message_id: 'boost-pages')
rows = []
capture = ->(name, user, action, setup = nil) do
  Current.user = user
  request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
  request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
  browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
  path = action == 'actions' ? "/rooms/#{room.id}/messages/#{message.id}/actions.json" : "/messages/#{message.id}/boosts#{action == 'new' ? '/new' : ''}"
  browser.get(path, headers: {'Cookie' => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", 'Turbo-Frame' => 'boosting'})
  Current.user = user
  body = action == 'actions' ? browser.response.body : GoldenBoostPagesController.renderer.new(http_host: 'campfire.test', https: false).render(template: "messages/boosts/#{action}", layout: false, assigns: browser.controller.view_assigns)
  rows << {name:, user_id: user.id, path:, setup:, action:, status: browser.response.status, content_type: browser.response.headers['Content-Type'], body:}
end
capture.call('index_empty', david, 'index')
capture.call('new_david', david, 'new')
boosts = [[david.id, '👍'], [david.id, '👍'], [jason.id, '👍'], [david.id, 'Legacy <&>'], [jason.id, ':github:']]
boosts.each { |booster_id, content| message.boosts.create!(booster_id:, content:) }
capture.call('index_mixed', david, 'index', 'boosts')
capture.call('actions_david', david, 'actions')
capture.call('actions_jason', jason, 'actions')
jason.update_columns(name: '<a href="/unsafe">Jason & "quoted"</a>')
capture.call('index_hostile_name', david, 'index', 'name')
capture.call('new_jason', jason, 'new')
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], message_id: message.id, boosts:, rows:) + "\n")
puts "WS8bm boost-pages oracle: #{rows.size} actual index/new/actions requests; complete fixed-token forms, distinct reactors and hostile tooltip names"
