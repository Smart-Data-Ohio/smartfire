HASHES={'app/controllers/rooms_controller.rb' => '53c7fd8b0619e478425d492f1be3abbc11affafa1b992dea09f8cd0b413c94eb', 'app/controllers/rooms/opens_controller.rb' => '932e1cc5ab97663253f5355cd2944f69779804a549d2ec14c62806c7f2717d0c', 'app/controllers/rooms/directs_controller.rb' => '46f1d745798c0a9003915c3a6c0b265d9f5079cad43812816b940a7decdcc460', 'app/controllers/rooms/closeds_controller.rb' => 'de11cf1268a4f84cb9d7d6b4dc972b4d6e27708404034ed4b6f133ec263973da'}
require 'json'
require 'digest'
HASHES.each { |path,hash|raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
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


def snapshot(client, before)
  flash=client.request.flash.to_hash
  next_flash=client.request.flash.to_session_value&.fetch('flashes',{}) || {}
  ActiveSupport::IsolatedExecutionState.clear
  path=client.response.headers['Location']
  id=path&.match(%r{/rooms/(\d+)(?:\?|$)})&.captures&.first
  room=id && Room.find(id)
  {status:client.response.status,location:path,flash:flash,next_flash:next_flash,count_delta:Room.directs.count-before,user_ids:room&.user_ids&.sort}
end
creates=[]
inputs=[[[149087659,712064548]],[[149087659],[712064548]],[149087659,149087659,712064548],149087659.75,nil,true,false,{id:149087659},[{id:149087659}],[nil,149087659], '149087659junk']
inputs.each do |value|
  before=Room.directs.count
  client.post('/rooms/directs',params:{user_ids:value},headers:{'X-CSRF-Token'=>token},as: :json)
  creates << snapshot(client,before).merge(input:value)
end
additions=[]
add_inputs=[[[773523953]],[[773523953,394959859]],[nil,773523953],{id:773523953},[{id:773523953}],nil,true,false,[773523953,773523953]]
add_inputs.each do |value|
  Current.user=user
  room=Rooms::Direct.create!(name:'Selection probe')
  room.memberships.grant_to(User.where(id:[127326141,149087659,712064548]))
  ActiveSupport::IsolatedExecutionState.clear
  client.post("/rooms/directs/#{room.id}/add_members",params:{user_ids:value},headers:{'X-CSRF-Token'=>token},as: :json)
  flash=client.request.flash.to_hash
  next_flash=client.request.flash.to_session_value&.fetch('flashes',{}) || {}
  ActiveSupport::IsolatedExecutionState.clear
  additions << {input:value,status:client.response.status,location:client.response.headers['Location'],flash:flash,next_flash:next_flash,user_ids:room.reload.user_ids.sort,notes:room.messages.where(system_note:true).map(&:plain_text_body),audits:AuditLog.where(target_type:'Room',target_id:room.id).pluck(:action,:details)}
end
predicates=[[[127326141,149087659]],[[127326141],149087659],[[127326141],[149087659]],[[[127326141,149087659]]]].map { |input| {input:input,user_ids:User.where(id:input).pluck(:id).sort,sql:User.where(id:input).to_sql} }
puts JSON.pretty_generate({reference: ENV.fetch('PARITY_REFERENCE_SHA'),creates:creates,additions:additions,predicates:predicates})
warn "Rails direct selection oracle: #{creates.size} create inputs, #{additions.size} member inputs; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
