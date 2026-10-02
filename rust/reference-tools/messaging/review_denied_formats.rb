require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user_id=127326141
room_id=654632876
message_id=Poll.find(1).message_id
Membership.where(user_id:user_id,room_id:room_id).delete_all
user=User.find(user_id)
session=user.sessions.where.not(two_factor_verified_at:nil).first!
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=session.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
requests=[
 ['get',"/rooms/#{room_id}/polls/1"], ['post',"/rooms/#{room_id}/polls"],
 ['post',"/rooms/#{room_id}/polls/1/vote"], ['delete',"/messages/#{message_id}/pin"],
 ['post','/saved'], ['post',"/rooms/#{room_id}/scheduled_messages"],
 ['post',"/rooms/#{room_id}/slash_commands"], ['get',"/autocompletable/slash_commands?room_id=#{room_id}"]
]
checked=0
['application/json','text/html','text/vnd.turbo-stream.html'].each do |accept|
 requests.each do |method,path|
  browser.public_send(method,path,params:JSON.generate(message_id:message_id),headers:headers.merge('Accept'=>accept,'Content-Type'=>'application/json'))
  raise "#{accept} #{method} #{path}: #{browser.response.status} #{browser.response.body.inspect}" unless browser.response.status==404 && browser.response.body.empty?
  checked+=1
 end
end
puts "REVIEW Rails denied response formats: #{checked}/24 empty 404 bodies"
