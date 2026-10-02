# Whole thread content through the actual authorized Rails show caller and GitHub provider.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
ActiveJob::Base.queue_adapter = :test
viewer=User.find(127326141)
thread=ChannelThread.find(8)
pr=thread.pull_request_thread.pull_request
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=viewer.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
rows=[]
[false,true,nil].each do |private_value|
 pr.update_columns(private:private_value)
 Current.reset
 browser.get("/rooms/#{thread.room_id}/threads/#{thread.id}",headers:headers)
 Current.user=viewer
 body=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(template:'channel_threads/show',layout:false,assigns:browser.controller.view_assigns)
 rows << {private:private_value,status:browser.response.status,body:body}
end
Current.reset
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',room_id:thread.room_id,thread_id:thread.id,pull_request_id:pr.id,rows:rows)+"\n")
puts "WS8bm GitHub thread page: #{rows.size} complete public/private/unknown show bodies; reference d7c7de92"
