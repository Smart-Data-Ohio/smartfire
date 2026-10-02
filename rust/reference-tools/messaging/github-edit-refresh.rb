# Actual human edits through the shared GitHub renderer; bodyless legacy and unchanged rich-text bodies.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,1,1,12)
ActionController::Base.allow_forgery_protection=false
Rails.application.env_config['action_dispatch.show_exceptions']=:all
ActiveJob::Base.queue_adapter=:test
now=Time.current
User.insert_all!([{id:811,name:'Oracle',role:1,created_at:now,updated_at:now}])
Room.insert_all!([{id:815,type:'Rooms::Closed',creator_id:811,name:'Cards',created_at:now,updated_at:now}])
Membership.insert_all!([{room_id:815,user_id:811,created_at:now,updated_at:now}])
Message.insert_all!([{id:818,room_id:815,creator_id:811,client_message_id:'card-parent',created_at:now,updated_at:now}])
ChannelThread.insert_all!([{id:817,room_id:815,creator_id:811,parent_message_id:818,name:'Discussion',messages_count:1,last_activity_at:now,created_at:now,updated_at:now}])
Message.insert_all!([{id:819,room_id:815,thread_id:817,creator_id:811,client_message_id:'card-thread',created_at:now,updated_at:now}])
pr=Github::PullRequest.create!(id:816,owner:'rails',repo:'rails',number:12,title:'Secret title',state:'open',private:false,
                             changed_files:{files:[{filename:'app/a.rb',status:'modified',additions:4,deletions:2}],total_count:3}.to_json)
Github::PullRequestReference.insert_all!([818,819].map { |id| {github_pull_request_id:816,message_id:id,created_at:now,updated_at:now} })
viewer=User.find(811)
session=viewer.sessions.create!(two_factor_verified_at:now)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'example.org','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=session.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'example.org'
rows=[]
[['bodyless_root',818,'/rooms/815/messages/818',nil],
 ['valid_root',818,'/rooms/815/messages/818','<p>https://github.com/rails/rails/pull/12</p>'],
 ['valid_thread',819,'/rooms/815/threads/817/messages/819','<p>https://github.com/rails/rails/pull/12</p>']].each do |name,id,path,body|
 Current.reset
 message=Message.find(id)
 message.body=body; message.body.save! if body
 pr.update_columns(fetched_at:nil,fetch_requested_at:nil)
 ActiveJob::Base.queue_adapter.enqueued_jobs.clear
 browser.patch(path,params:{message:{embeds_suppressed:true}}.to_json,headers:headers.merge('Content-Type'=>'application/json','Accept'=>'text/html'))
 message.reload; pr.reload
 rows << {name:name,id:id,path:path,body_before:body,status:browser.response.status,body:browser.response.body,
          location:browser.response.headers['Location'],content_type:browser.response.headers['Content-Type'],
          fetch_jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.count { |job| job[:job]==Github::FetchPullRequestJob },
          claimed:pr.fetch_requested_at.present?,saved_body:message.rich_text_body&.body&.to_html,
          suppressed:message.embeds_suppressed,edited_at:message.edited_at&.iso8601(6)}
end
Current.reset
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:rows)+"\n")
puts "WS8bm GitHub edit refresh: #{rows.size} actual Rails requests; bodyless legacy and unchanged root/thread refresh claims"
