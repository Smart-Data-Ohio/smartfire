# The actual root/nested controller edits and their publisher frames.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
viewer=User.find(127326141);room=Room.find(486777696)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=viewer.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
root=room.messages.create!(creator:viewer,markdown_source:'Original',client_message_id:'provider-root')
thread=ChannelThread.create!(room:room,creator:viewer,name:'Provider thread')
reply=thread.post_message!(creator:viewer,attributes:{markdown_source:'Original',client_message_id:'provider-reply'})
pr=Github::PullRequest.for_reference(owner:'rails',repo:'rails',number:511)
pr.update!(private:false,title:'Seeded PR card',state:'open',fetched_at:Time.current)
pr.update_column(:fetch_requested_at,nil)
frames=[]
ActionCable.server.singleton_class.prepend(Module.new do
 define_method(:broadcast){|stream,payload,**options|frames << {stream:stream,payload:payload};super(stream,payload,**options)}
end)
rows=[]
[ ['root_add',root,{markdown_source:'see https://github.com/rails/rails/pull/511'}],
  ['root_remove',root,{markdown_source:'never mind'}],
  ['thread_add',reply,{markdown_source:'see https://x.com/jack/status/112233'}],
  ['thread_remove',reply,{markdown_source:'never mind'}],
  ['legacy_resync',root,{body:'<div>see https://github.com/rails/rails/pull/512 and https://x.com/jack/status/424242</div>'}]
].each_with_index do |(name,message,input),index|
 travel_to Time.utc(2026,3,2,16)+index+1
 frames.clear
 path=message.thread_id ? "/rooms/#{room.id}/threads/#{thread.id}/messages/#{message.id}" : "/rooms/#{room.id}/messages/#{message.id}"
 browser.patch(path,params:{message:input},headers:headers)
 message.reload
 rows << {name:name,time:Time.current.iso8601,path:path,input:input,status:browser.response.status,location:browser.response.headers['Location'],
  github:message.github_pull_requests.map{|pr|[pr.owner,pr.repo,pr.number]},twitter:message.twitter_posts.map(&:post_id),
  frames:frames.select{|frame|frame[:stream].end_with?(':messages')}}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],root_id:root.id,thread_id:thread.id,reply_id:reply.id,rows:rows)+"\n")
puts "WS8bm provider declarations: #{rows.size} actual root/thread edits; #{rows.sum{|r|r[:frames].size}} exact message publisher frames; PR/Twitter add/remove and legacy reference synchronization"
