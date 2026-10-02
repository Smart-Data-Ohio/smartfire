require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
ActiveJob::Base.queue_adapter=:test
Rails.application.config.hosts.clear
user=User.find(127326141);room=Room.find(486777696)
room.memberships.update_all(unread_at:nil)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
frames=[]
ActionCable.server.singleton_class.prepend(Module.new do
 define_method(:broadcast){|stream,payload,**options|frames << {stream:stream,payload:payload};super(stream,payload,**options)}
end)
browser.post("/rooms/#{room.id}/messages.turbo_stream",params:{message:{markdown_source:'Recipient marker',client_message_id:'recipient-marker'}},headers:headers)
# Human viewer streams only. Bender's bot delivery is checked by its owner.
selected=frames.select{|frame|[127326141,149087659,712064548].any?{|id|frame[:stream]=="user_#{id}_unreads"}}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',status:browser.response.status,frames:selected)+"\n")
puts "WS8bm root recipients: #{selected.size} actual Rails personal unread frames; room members only"
