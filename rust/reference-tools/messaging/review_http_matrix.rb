require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user_id=127326141
room_id=654632876
message_id=Poll.find(1).message_id
cases=[]
['Rooms::Open','Rooms::Closed','Rooms::Direct','Rooms::Board'].each do |kind|
  [[0,0],[1,0],[2,0],[0,1],[0,2]].each do |role,status|
    [true,false].each do |member|
      ActiveRecord::Base.transaction(requires_new:true) do
        User.where(id:user_id).update_all(role:role,status:status)
        Room.unscoped.where(id:room_id).update_all(type:kind)
        Membership.where(user_id:user_id,room_id:room_id).delete_all unless member
        user=User.find(user_id)
        session=user.sessions.where.not(two_factor_verified_at:nil).first!
        request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
        request.cookie_jar.signed[:session_token]=session.token
        headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
        browser=ActionDispatch::Integration::Session.new(Rails.application)
        browser.host! 'campfire.test'
        requests=[
          ['get',"/rooms/#{room_id}/polls/1",{}],
          ['post',"/rooms/#{room_id}/polls",{poll:{question:'Review?',options:['Only']}}],
          ['post',"/rooms/#{room_id}/polls/1/vote",{option_ids:[]}],
          ['delete',"/messages/#{message_id}/pin",{}],
          ['post','/saved',{message_id:message_id,saved_item:{status:'invalid-review-status'}}],
          ['post',"/rooms/#{room_id}/scheduled_messages",{scheduled_message:{markdown_source:'',send_at:'2026-03-03T16:00:00Z'}}],
          ['post',"/rooms/#{room_id}/slash_commands",{command:'unknown-review-command'}],
          ['get',"/autocompletable/slash_commands?room_id=#{room_id}",{}],
          ['get',"/autocompletable/users?room_id=#{room_id}&query=Jason",{}],
          ['get',"/autocompletable/icons?query=github",{}],
          ['get',"/rooms/#{room_id}/files",{}],
          ['get',"/rooms/#{room_id}/message_links/987654321",{}],
          ['get','/searches?q=from%3A%40david',{}]
        ]
        outputs=requests.map do |method,path,input|
          browser.public_send(method,path,params:input,headers:headers.dup,as: :json)
          response=browser.response
          {method:method,path:path,input:input,status:response.status,body:(response.media_type=='application/json'||response.body.empty? ? response.body : nil),compare_body:response.media_type=='application/json'||response.body.empty?,location:response.location}
        end
        cases << {kind:kind,role:role,user_status:status,member:member,requests:outputs}
        raise ActiveRecord::Rollback
      end
    end
  end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(cases)+"\n")
puts "REVIEW Rails role/room matrix: #{cases.size} combinations; #{cases.sum{|c|c[:requests].size}} responses recorded"
