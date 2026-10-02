require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
viewer=User.find(127326141);creator=User.find(149087659);room=Room.find(486777696)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=viewer.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
rows=[]
threads=[['Recent closed',1,true,false],['Locked',2,true,true],['Stale',3,false,false],['Old closed',4,true,false],['Active',0,false,false]].map do |name,hours,closed,locked|
 thread=ChannelThread.create!(room:room,creator:creator,name:name,auto_archive_after_minutes:60)
 time=Time.current-hours.hours
 thread.update_columns(last_activity_at:time,closed_at:closed ? time : nil,locked_at:locked ? time : nil)
 thread
end
path="/rooms/#{room.id}/threads.json?state=closed"
browser.get(path,headers:headers)
rows << {name:'closed',method:'get',path:path,params:{},status:browser.response.status,body:browser.response.body}
browser.post('/rooms/186869642/threads.json',params:{thread:{name:'Not permitted'}},headers:headers,as: :json)
rows << {name:'direct_denied',status:browser.response.status}
# Separate room state matches the separate Rust test app.
threads.each(&:destroy!)
locked=ChannelThread.create!(room:room,creator:creator,name:'Locked stale',auto_archive_after_minutes:60)
locked.lock_conversation!
stale=ChannelThread.create!(room:room,creator:creator,name:'Stale sibling',auto_archive_after_minutes:60)
live=ChannelThread.create!(room:room,creator:creator,name:'Live sibling')
[locked,stale].each{|thread|thread.update_columns(last_activity_at:Time.current-2.hours)}
browser.patch("/rooms/#{room.id}/threads/#{locked.id}.json",params:{thread:{status:'active'}},headers:headers,as: :json)
rows << {name:'unlock',status:browser.response.status,active:locked.reload.active?,last_activity_at:locked.last_activity_at.utc.iso8601(3)}
browser.post("/rooms/#{room.id}/threads/#{live.id}/messages.json",params:{message:{markdown_source:'Hello',client_message_id:'sweep-trigger'}},headers:headers,as: :json)
rows << {name:'sweep',status:browser.response.status,closed_at:stale.reload.closed_at.utc.iso8601(3)}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',closed_ids:threads.first(4).map(&:id),rows:rows)+"\n")
puts 'WS8bm thread declarations: closed/locked/stale ordering, direct refusal, stale unlock and sibling sweep through real Rails requests'
