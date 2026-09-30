require 'json'
user=User.find(127326141)
session=user.sessions.create!(two_factor_verified_at:Time.current)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:session.token,httponly:true,same_site: :lax}
client=ActionDispatch::Integration::Session.new(Rails.application)
client.host! 'campfire.test'
client.cookies['session_token']=request.cookie_jar[:session_token]
client.get('/users/me/profile')
ActiveSupport::IsolatedExecutionState.clear
token=Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')['content']
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,message,**_| frames << {stream:stream,html:message} if stream.end_with?(':rooms') }
operations=[]
[[486777696,'muted'],[486777696,'everything'],[486777696,'invisible'],[486777696,'mentions'],[699448329,'muted'],[699448329,'everything']].each do |room_id,level|
  frames.clear
  client.put("/rooms/#{room_id}/involvement",params:{involvement:level},headers:{'X-CSRF-Token'=>token,'Accept'=>'application/json'})
  raise "HTTP failure #{client.response.status}" unless client.response.status==200
  ActiveSupport::IsolatedExecutionState.clear
  operations << {room_id:room_id,level:level,frames:frames.map(&:dup)}
end
puts JSON.pretty_generate(operations)
warn "Rails involvement: #{operations.size} HTTP transitions, #{operations.sum { |o|o[:frames].size }} recipient frames; reference d7c7de92"
