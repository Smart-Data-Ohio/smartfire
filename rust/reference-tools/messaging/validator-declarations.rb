require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
ActiveJob::Base.queue_adapter=:test
Rails.application.config.hosts.clear
user=User.find(127326141);room=Room.find(486777696)
source=room.messages.create!(creator:user,markdown_source:'Off-page source',client_message_id:'validator-source')
50.times{|index|room.messages.create!(creator:user,markdown_source:"Filler #{index}",client_message_id:"validator-filler-#{index}")}
reply=room.messages.create!(creator:user,markdown_source:'Reply',reply_to_message:source,client_message_id:'validator-reply')
card=room.messages.create!(creator:user,markdown_source:'https://github.com/rails/rails/pull/530',client_message_id:'validator-card')
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
path="/rooms/#{room.id}/messages"
rows=[]
capture=->(name,extra={}) do
 browser.get(path,headers:headers.merge(extra))
 rows << {name:name,status:browser.response.status,etag:browser.response.headers['ETag']}
 browser.response.headers['ETag']
end
first=capture.call('first')
capture.call('unchanged',{'If-None-Match'=>first})
travel_to Time.utc(2026,3,2,16,0,10)
browser.patch("/rooms/#{room.id}/messages/#{source.id}.json",params:{message:{markdown_source:'Edited off-page source'}},headers:headers,as: :json)
second=capture.call('source_edit',{'If-None-Match'=>first})
travel_to Time.utc(2026,3,2,16,0,20)
card.github_pull_requests.sole.update!(private:false,title:'Fetched card',fetched_at:Time.current)
capture.call('card_fetch',{'If-None-Match'=>second})
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],source_id:source.id,reply_id:reply.id,card_id:card.id,rows:rows)+"\n")
puts 'WS8bm validator declarations: actual page 304, then byte-exact changed ETags after off-page source edit and card fetch without a message touch'
