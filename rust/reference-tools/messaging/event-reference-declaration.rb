require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user=User.find(127326141);room=Room.find(654632876);event=Event.find(390339825)
message=room.messages.create!(creator:user,body:'<div>no links here</div>',client_message_id:'legacy-resync-event')
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
input={body:"<div>see https://github.com/rails/rails/pull/512 and https://x.com/jack/status/424242 and /rooms/#{room.id}/events/#{event.id}</div>"}
path="/rooms/#{room.id}/messages/#{message.id}"
browser.patch(path,params:{message:input},headers:headers)
message.reload
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],message_id:message.id,input:input,path:path,status:browser.response.status,location:browser.response.headers['Location'],events:message.events.map(&:id))+"\n")
raise 'missing Rails event reference' unless message.events==[event]
puts 'WS8bm event owner probe: actual legacy PATCH; Rails synchronizes the event reference (WS14e)'
