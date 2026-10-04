require "json"
require "stringio"
load File.join(__dir__, "providers.rb")
base = JSON.parse(File.read(ARGV.fetch(0)))
user = User.find(127326141)
Current.user = user
room = Room.find(486777696)
last_id = Message.maximum(:id)
event = Event.create!(room: room, organizer: user, title: 'Populated event <&>', starts_at: Time.current + 25.hours, time_zone: 'America/New_York', venue: Room.find(699448331), meet_link: 'https://meet.google.com/abc-defg-hij?x=1&y=2')
Message.where('id > ?', last_id).each { |m| m.update_columns(client_message_id: "provider-batch-announcement-#{m.id}") }
connection = ActiveRecord::Base.connection
base['rows']['events'] = connection.select_all("SELECT * FROM events WHERE id=#{event.id}").to_a
%w[messages action_text_rich_texts event_references].each do |table|
 where = table == 'messages' ? "id > #{last_id}" : table == 'event_references' ? "message_id > #{last_id}" : "record_type='Message' AND record_id > #{last_id}"
 base['rows'][table] ||= []
 base['rows'][table].concat(connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a)
end
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>'campfire.test', "rack.input"=>StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers = { 'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
renderer = ApplicationController.renderer.new(http_host: 'campfire.test', https: false)
messages = []
pages = []
[4,16].each do |size|
 Rails.application.executor.wrap do
  Current.reset
  Current.user = user
  (messages.size...size).each do |i|
   messages << room.messages.create!(creator: user, client_message_id: "provider-populated-#{i}", markdown_source: "populatedquery https://github.com/provider-owner/repo-1/pull/2 /rooms/#{room.id}/events/#{event.id}")
  end
 end
 browser.get('/searches?q=populatedquery', headers: headers.dup)
 queries = []
 subscriber = ->(*args) { p=args.last; queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i) }
 ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record') { browser.get('/searches?q=populatedquery', headers: headers.dup) }
 raise "search status #{browser.response.status}" unless browser.response.successful?
 cards = Rails.application.executor.wrap do
  Current.reset
  messages.map do |m|
   m.reload
   Message.preload_rendering_details([m])
   { message_id: m.id, github: renderer.render(partial: 'github/pull_requests/cards', locals: {message: m}), events: renderer.render(partial: 'rooms/events/cards', locals: {message: m}) }
  end
 end
 pages << { size: size, reads: queries.size, cards: cards }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], rows: base['rows'], event_id: event.id, pages: pages)+"\n")
puts "WS8bm2 populated provider Rails: #{pages.map { |p| "#{p[:size]} messages=#{p[:reads]} reads" }.join('; ')}; #{pages.sum { |p| p[:cards].size }*2} GitHub/event containers"
