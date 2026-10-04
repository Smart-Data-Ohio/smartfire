require "json"
ActionController::Base.allow_forgery_protection=false; Rails.application.config.hosts.clear
user=User.find(127326141); Current.user=user; room=Room.find(486777696)
agent=Agent.find_by!(user_id:394959859)
AgentSlashCommand.create!(agent:,room:,name:"deploy")
request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={"Cookie"=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application); browser.host! "campfire.test"
browser.post("/rooms/#{room.id}/slash_commands",params:{text:"/deploy staging"},headers:,as: :json)
event=agent.agent_events.where(event_type:"slash_command").order(:id).last!
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],status:browser.response.status,body:browser.response.body,event:event.attributes.slice("agent_id","room_id","actor_id","event_type","outcome","metadata"))+"\n")
puts "WS8bm2 agent command: 1 real Rails HTTP invocation with persisted event"
