# Test-fixture operations copied from the pinned original, never a public route.
require 'json'
require 'webmock'
include WebMock::API
WebMock.enable!
WebMock.disable_net_connect!(allow_localhost: true)
request = JSON.parse(ARGV.fetch(0))
case request.fetch('action')
when 'setup'
  david = User.find(request.fetch('user'))
  if request.fetch('connect_calendar',false)
    GoogleAccount.create!(user: david, email: "#{david.name.parameterize}@gmail.test",
      refresh_token: "refresh-token-#{david.id}", access_token: "access-token-#{david.id}",
      access_token_expires_at: 1.hour.from_now)
  end
  attachment_body=ApplicationController.render partial: 'users/mention', locals: {user:david}
  mention="<action-text-attachment sgid=\"#{david.attachable_sgid}\" content-type=\"application/vnd.campfire.mention\" content=\"#{attachment_body.gsub('"','&quot;')}\"></action-text-attachment>"
  puts({ready:true,mention:mention}.to_json)
else
  raise 'model/broadcast fixtures must run in the actual rendering process'
end
