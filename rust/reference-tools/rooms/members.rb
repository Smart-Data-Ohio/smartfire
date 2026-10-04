# Real HTTP JSON bytes from our pinned Rails and owner fact readers.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
raise 'members source drift' unless Digest::SHA256.file(Rails.root.join('app/controllers/rooms/members_controller.rb')).hexdigest == '64ca2a50c0c10ce2ba94e054d522e1cb64871c8f2da2248244a22c1e11b05ff9'
travel_to Time.utc(2026,3,2,16)
rows=[]
ActiveRecord::Base.transaction do
  [127326141,149087659].each do |id|
    user=User.find(id)
    session=user.sessions.create!(two_factor_verified_at:Time.current)
    request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
    request.cookie_jar.signed.permanent[:session_token]={value:session.token,httponly:true,same_site: :lax}
    client=ActionDispatch::Integration::Session.new(Rails.application)
    client.host! 'campfire.test'
    client.cookies['session_token']=request.cookie_jar[:session_token]
    [654632876,201306877,186869642].each do |room_id|
      ActiveSupport::IsolatedExecutionState.clear
      client.get "/rooms/#{room_id}/members.json"
      raise "HTTP failed: #{client.response.status}" unless client.response.status==200
      rows << {viewer_id:id,room_id:room_id,body:client.response.body,headers:client.response.headers.slice('Cache-Control','Pragma','ETag')}
    end
  end
  raise ActiveRecord::Rollback
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),rows:rows)
warn "Rails room members: #{rows.size} complete HTTP JSON goldens; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
