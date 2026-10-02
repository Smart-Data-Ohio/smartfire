require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user=User.find(127326141)
room=Room.find(486777696)
session=user.sessions.where.not(two_factor_verified_at:nil).first!
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=session.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
results=[]
[4,16].each do |n|
  (Message.where(client_message_id:'review-query-count').count...n).each do |i|
    room.messages.create!(creator:user,markdown_source:'reviewquerycount common',client_message_id:"review-query-count-#{n}-#{i}")
  end
  browser.get('/searches?q=reviewquerycount',headers:headers)
  selects=[]
  sub=ActiveSupport::Notifications.subscribe('sql.active_record') do |name,start,finish,id,payload|
    selects << payload[:sql] if !payload[:cached] && payload[:sql].lstrip.start_with?('SELECT','WITH')
  end
  browser.get('/searches?q=reviewquerycount',headers:headers)
  ActiveSupport::Notifications.unsubscribe(sub)
  results << {messages:n,status:browser.response.status,selects:selects}
  puts "REVIEW Rails warm search #{n} messages: #{selects.size} SELECT/WITH executions"
  Message.where("client_message_id LIKE 'review-query-count-%'").destroy_all
end
File.write(ARGV.fetch(0),JSON.pretty_generate(results)+"\n")
