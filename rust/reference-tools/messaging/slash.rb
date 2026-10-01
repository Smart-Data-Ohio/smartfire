require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
user=User.find_by!(email_address:'david@37signals.com')
room=user.rooms.find_by!(name:'All Talk')
user.update!(time_zone:'America/New_York')
Current.user=user
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
# Render before IntegrationSession resets the runner's Current attributes.
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
play=(Sound.names+['unknown','','BELL','bell two','bell\nbell']).map.with_index do |name,i|
 source="/play #{name}".strip
 message=room.messages.create!(creator:user,markdown_source:source,client_message_id:"slash-play-#{i}")
 {source:,client_message_id:message.client_message_id,html:renderer.render(partial:'messages/presentation',locals:{message:})}
end
texts=['/shrug ship it','/me testing','/play bell','/play 56k','/play unknown','/play','/poll','/event','/event Launch party','/status 🚂 On a train','/dnd 30m','/dnd off','/ooo tomorrow Working remote','/ooo off','/remind in 20 minutes review deploy','/frobnicate',nil,42,['/poll'],{'a'=>'b'}]
steps=texts.map do |text|
 browser.post "/rooms/#{room.id}/slash_commands",params:{text:},headers:headers.dup,as: :json
 {text:,status:browser.response.status,body:browser.response.body}
end
svg='<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><path d="M0 0h64v64H0z"/></svg>'
%w[acme globex].each{|name|WorkspaceIcon.create!(name:,title:name.capitalize,creator:user,image:{io:StringIO.new(svg),filename:"#{name}.svg",content_type:'image/svg+xml'})}
agent=Agent.first!
AgentSlashCommand.create!(agent:,room:,name:'deploy',description:'Ship it',takes_arguments:true)
AgentSlashCommand.create!(agent:,room:,name:'zero',takes_arguments:false)
thread=room.channel_threads.create!(creator:user,name:'Side chat')
paths=['/autocompletable/icons.json?q=open','/autocompletable/icons.json?q=fire','/autocompletable/icons.json?q=','/autocompletable/icons.json?q=zzz_no_such_icon','/autocompletable/icons.json?q=acme','/autocompletable/icons.json?custom=0','/autocompletable/icons.json?q=&query=fire','/autocompletable/icons.json?q=FIRE','/autocompletable/icons.json?q=gpt','/autocompletable/icons.json?q=%20%3Afire%3A%20',"/autocompletable/slash_commands.json?room_id=#{room.id}","/autocompletable/slash_commands.json?room_id=#{room.id}&query=STA", "/autocompletable/slash_commands.json?room_id=#{room.id}&thread_id=#{thread.id}",'/autocompletable/users.json?query=da',"/autocompletable/users.json?query=da&room_id=#{room.id}",'/autocompletable/users.json?filter=da','/autocompletable/users.json?page=2']
pickers=paths.map do |path|
 browser.get path,headers:headers.merge('Accept'=>'application/json')
 {path:,status:browser.response.status,body:browser.response.body,total:browser.response.headers['X-Total-Count'],link:browser.response.headers['Link']}
end
formats=['/autocompletable/users','/autocompletable/icons',"/autocompletable/slash_commands?room_id=#{room.id}"].map do |path|
 browser.get path,headers:headers.merge('Accept'=>'text/html')
 {path:,status:browser.response.status,content_type:browser.response.media_type}
end
readiness=[{}, {'LIVEKIT_URL'=>' '}, {'LIVEKIT_URL'=>'http://Gateway', 'LIVEKIT_INTERNAL_URL'=>'ws://gateway'}, {'LIVEKIT_URL'=>'https://gateway', 'LIVEKIT_INTERNAL_URL'=>'wss://gateway:443'}, {'LIVEKIT_URL'=>'wss://gateway', 'LIVEKIT_INTERNAL_URL'=>'http://livekit:7880'}, {'LIVEKIT_URL'=>'http://gateway:81', 'LIVEKIT_INTERNAL_URL'=>'ws://gateway'}, {'LIVEKIT_URL'=>'ftp://gateway'}, {'LIVEKIT_URL'=>'/gateway'}, {'LIVEKIT_URL'=>'not a url'}, {'LIVEKIT_API_KEY'=>nil}, {'LIVEKIT_URL'=>'HTTPS://gateway'}, {'LIVEKIT_URL'=>'http://[::1]:81', 'LIVEKIT_INTERNAL_URL'=>'http://[::1]:80'}].map do |overrides|
 env={'LIVEKIT_URL'=>'https://gateway', 'LIVEKIT_INTERNAL_URL'=>'http://livekit:7880', 'LIVEKIT_API_KEY'=>'fixture-key', 'LIVEKIT_API_SECRET'=>'fixture-secret', 'LIVEKIT_GATEWAY_SECRET'=>'fixture-gateway'}.merge(overrides)
 previous=Huddle::REQUIRED_ENVIRONMENT.to_h{|k|[k,ENV[k]]}
 begin
  env.each{|k,v|ENV[k]=v}
  {env:, configured:Huddle.configured?}
 ensure
  previous.each{|k,v|ENV[k]=v}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',steps:,pickers:,play:,formats:,agent_id:agent.id,thread_id:thread.id,readiness:)+"\n")
puts "WS8bm2 slash Rails oracle: #{steps.size} dispatch responses; #{pickers.size} picker responses; #{play.size} play presentation fragments; #{formats.size} format responses; #{readiness.size} huddle readiness cases"
