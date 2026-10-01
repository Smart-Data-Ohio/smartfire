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

rows=[]
inputs=[nil,false,true,42,1.25,"  Team <&>  ","\u00a0",["ignored"],{name:"ignored"},"é"*100,"é"*101]
inputs.each do |value|
  Current.user=user
  room=Rooms::Direct.create!(name:'Old')
  room.memberships.grant_to(User.where(id:[127326141,149087659,712064548]))
  ActiveSupport::IsolatedExecutionState.clear
  before={messages:Message.count,audits:AuditLog.count}
  client.patch("/rooms/directs/#{room.id}",params:{room:{name:value}},headers:{'X-CSRF-Token'=>token,'Accept'=>'text/html'},as: :json)
  body=Nokogiri::HTML(client.response.body)
  flash=client.request.flash.to_session_value&.fetch('flashes',{}) || {}
  ActiveSupport::IsolatedExecutionState.clear
  rows << {input:value,status:client.response.status,location:client.response.headers['Location'],next_flash:flash,name:room.reload.name,notes:room.messages.where(system_note:true).map(&:plain_text_body),error_value:body.at_css('.field_with_errors input')&.[]('value'),delta:{messages:Message.count-before[:messages],audits:AuditLog.count-before[:audits]}}
end
puts JSON.pretty_generate({reference:'d7c7de92',renames:rows})
warn "Rails direct rename: #{rows.size} scalar, collection, Unicode and invalid HTTP cases; reference d7c7de92"
