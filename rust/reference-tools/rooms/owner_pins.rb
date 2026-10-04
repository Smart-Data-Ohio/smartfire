require 'json'
require 'stringio'
Current.user=User.find(127326141)
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
rooms=[Room.find(699448326),Room.find(186869642)]
panels=rooms.map do |room|
  empty=renderer.render(partial:'rooms/pins/panel',locals:{room:})
  2.times do |i|
    message=room.messages.create!(creator:Current.user,markdown_source:"panel pin #{i}",client_message_id:"panel-#{room.id}-#{i}")
    MessagePin.create!(room:,message:,pinner:Current.user)
  end
  {room_id:room.id,empty:,populated:renderer.render(partial:'rooms/pins/panel',locals:{room:})}
end
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=Current.user.sessions.where.not(two_factor_verified_at:nil).first!.token
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
browser.get '/users/me/sidebar',headers:{'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
raise "sidebar status #{browser.response.status}" unless browser.response.status==200
links=%w[/saved /scheduled_messages].map do |path|
  html=browser.response.body
  attribute=html.index("data-workspace-destination=\"#{path}\"") or raise "missing sidebar #{path}"
  start=html.rindex('<a ',attribute);finish=html.index('</a>',attribute)+4
  {path:,html:html[start...finish]}
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),panels:,links:)
warn "WS8bm2 panels Rails oracle: #{panels.size*2} pin panels; #{links.size} sidebar links"
