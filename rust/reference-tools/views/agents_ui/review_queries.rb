# Actual SQL growth at the pinned Rails controllers; each request starts without a query cache.
require 'action_dispatch/testing/integration'
require 'json'
ActiveRecord::Base.logger = nil
ApplicationController.allow_forgery_protection = false
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
david = User.find(127326141)
room = Room.find(486777696)
request = lambda do |path, table, accept = 'application/json'|
  queries = []
  subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*, payload|
    sql = payload[:sql]
    queries << sql if sql.match?(/\ASELECT/i) && sql[/\bFROM\s+"?([a-z_]+)/i, 1] == table && !payload[:cached]
  end
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! 'campfire.test'
  browser.get(path, headers: { 'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'Accept' => accept, 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0' })
  raise "#{path}: #{browser.response.status}" unless browser.response.status == 200
  queries.size
ensure
  ActiveSupport::Notifications.unsubscribe(subscriber)
  ActiveSupport::ExecutionContext.clear
end
%w[application/json text/html text/vnd.turbo-stream.html].each do |accept|
  [1, 100].each do |size|
    ActivityItem.delete_all
    room.messages.order(:id).limit(size).each do |message|
      ActivityItem.create!(user: david, source: message, event_type: 'mention')
    end
    count = request.call('/activity', 'messages', accept)
    puts "Rails inbox #{accept}: #{size} items; message_selects=#{count}; expected=1"
    raise "Message source N+1: #{count}" unless count == 1
  end
end
[0, 10].each do |additional|
  additional.times do |index|
    bot = User.create!(name: "Polling bot #{index}", role: :bot, status: :active)
    Agent.create!(user: bot, owner: david)
    Membership.create!(room: room, user: bot)
  end
  count = request.call("/rooms/#{room.id}/members.json", 'agents')
  puts "Rails member polling +#{additional} bots: agent_selects=#{count}; expected=1"
  raise "Agent N+1: #{count}" unless count == 1
end
