# Real pin-only refresh response, composed from unchanged Rails owner partials.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
raise 'refresh source drift' unless Digest::SHA256.file(Rails.root.join('app/controllers/rooms/refreshes_controller.rb')).hexdigest=='43f3d103e2d37c7e04af0f6117ebec56a86d43f9276d9fec961b802796822344'
# Rendering instrumentation fixes only request-secret entropy, as the existing shell goldens do.
class Rooms::RefreshesController
  def form_authenticity_token(form_options: {}) = 'GLOBAL'
end
travel_to Time.utc(2026,3,2,16)
rows=[]
ActiveRecord::Base.transaction do
  user=User.find(127326141);room=Room.find(486777696)
  session=user.sessions.create!(two_factor_verified_at:Time.current)
  request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  request.cookie_jar.signed.permanent[:session_token]={value:session.token,httponly:true,same_site: :lax}
  client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test';client.cookies['session_token']=request.cookie_jar[:session_token]
  message=room.root_messages.ordered.first
  [false,true].each do |pin|
    MessagePin.pin!(message:message,pinner:user) if pin
    room.update_columns(pins_changed_at:Time.current+1.minute)
    ActiveSupport::IsolatedExecutionState.clear
    client.get "/rooms/#{room.id}/refresh.turbo_stream",params:{since:(Time.current.to_f*1000).to_i}
    raise "bad refresh #{client.response.status}" unless client.response.status==200
    rows << {pinned:pin,message_id:message.id,body:client.response.body}
  end
  raise ActiveRecord::Rollback
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),rows:rows)
warn "Rails room pin refresh: #{rows.size} complete HTTP Turbo goldens; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
