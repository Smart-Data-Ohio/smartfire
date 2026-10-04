require 'json'
require "digest"
{
  "app/controllers/rooms/involvements_controller.rb" => "23a1d4390c398b4f3c428e182401796ea337d701b182ed708d9a18aba3f13e45",
}.each { |path, hash| raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
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
warn "Rails involvement: #{operations.size} HTTP transitions, #{operations.sum { |o|o[:frames].size }} recipient frames; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
