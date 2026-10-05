# Original Users::SidebarsControllerTest huddle declarations, through the Rails router.
require 'json'
Rails.application.config.hosts.clear
Rails.logger = ActiveSupport::Logger.new($stderr)
ActionView::Base.logger = Rails.logger
ENV.update('LIVEKIT_URL'=>'wss://huddle.example.test','LIVEKIT_INTERNAL_URL'=>'ws://livekit.example.test:7880','LIVEKIT_API_KEY'=>'fixture-api-key','LIVEKIT_API_SECRET'=>'fixture-api-secret','LIVEKIT_GATEWAY_SECRET'=>'fixture-gateway-secret')
viewer=User.find(127326141)
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:viewer.sessions.first.token,httponly:true,same_site: :lax}
browser.cookies['session_token']=request.cookie_jar[:session_token]
room=Room.find(486777696)
# IDs are resolved through membership identity rather than assuming fixture insertion order.
direct=Rooms::Direct.find_or_create_for([viewer,User.find(149087659)])
board=Rooms::Board.create_for({name:'Launch',creator:viewer},users:[viewer,User.find(773523953)])
group=Rooms::Direct.create_for({creator:viewer},users:[viewer,User.find(149087659),User.find(712064548)])
def reset_grants
 HuddleGrant.delete_all
end
def grant(user_id,room,seen=true)
 u=User.find(user_id);s=u.sessions.first || u.sessions.create!(user_agent:'Original fixture')
 HuddleGrant.issue!(session:s,membership:room.memberships.find_by!(user:u)).tap{|g|g.update_columns(last_seen_at:Time.current) if seen}
end
def get_row(browser,room)
 browser.get '/users/me/sidebar'
 ActiveSupport::IsolatedExecutionState.clear
 dom=Nokogiri::HTML(browser.response.body)
 {status:browser.response.status,html:dom.at_css("#list_#{room.model_name.singular}_#{room.id}")&.to_html,room_id:room.id}
end
rows=[]
reset_grants;grant(viewer.id,room);grant(149087659,room);rows << {name:'channel',response:get_row(browser,room)}
reset_grants;grant(viewer.id,board);grant(773523953,board);rows << {name:'board',response:get_row(browser,board)}
reset_grants;grant(149087659,direct);rows << {name:'direct',response:get_row(browser,direct)}
reset_grants;rows << {name:'quiet_channel',response:get_row(browser,room)};rows << {name:'quiet_direct',response:get_row(browser,direct)}
reset_grants;grant(149087659,group);rows << {name:'group',response:get_row(browser,group)}
# Real cached collection key sequence: quiet -> unrelated rename -> first participant.
reset_grants;g=grant(149087659,direct,false)
Rails.cache=ActiveSupport::Cache::MemoryStore.new
ActionView::PartialRenderer.collection_cache=Rails.cache
ActionController::Base.perform_caching=true
rows << {name:'cache_before',response:get_row(browser,direct)}
User.find(149087659).update!(name:'Jordan')
rows << {name:'cache_rename',response:get_row(browser,direct)}
g.record_seen!
rows << {name:'cache_join',response:get_row(browser,direct)}
# Each following original declaration starts without the cache-test override.
# Otherwise cached configured direct rows leak voice stacks into this case.
Rails.cache=ActiveSupport::Cache::NullStore.new
ActionView::PartialRenderer.collection_cache=Rails.cache
ActionController::Base.perform_caching=false
reset_grants;grant(149087659,room);ENV.delete('LIVEKIT_GATEWAY_SECRET')
browser.get '/users/me/sidebar';ActiveSupport::IsolatedExecutionState.clear
dom=Nokogiri::HTML(browser.response.body)
rows << {name:'unconfigured',response:{status:browser.response.status,shared_stacks:dom.css('#shared_rooms .voice-stack').size,direct_stacks:dom.css('#direct_rooms .voice-stack').size,presence:dom.css('[data-controller~="huddle-presence"]').size}}
# Original two-size mixed quiet-room SELECT and one-grants-read assertions.
ENV['LIVEKIT_GATEWAY_SECRET']='fixture-gateway-secret'
reset_grants
User.find(149087659).update!(name:'Jason')
def quiet_room(type,name,viewer)
 type.create_for({name:name,creator:viewer},users:[viewer])
end
2.times{|i|quiet_room(Rooms::Closed,"Quiet #{i}",viewer)}
Rooms::Direct.create_for({creator:viewer},users:[viewer,User.find(773523953)])
quiet_room(Rooms::Board,'Quiet board',viewer)
2.times{|i|quiet_room(Rooms::Stage,"Quiet stage #{i}",viewer)}
browser.get '/users/me/sidebar';ActiveSupport::IsolatedExecutionState.clear
def measured(browser)
 queries=[]
 sub=ActiveSupport::Notifications.subscribe('sql.active_record'){|*,p| queries << p[:sql] if p[:sql].start_with?('SELECT') && p[:name]!='SCHEMA'}
 browser.get '/users/me/sidebar';ActiveSupport::IsolatedExecutionState.clear
 {status:browser.response.status,selects:queries.size,grants:queries.count{|q|q.include?('FROM "huddle_grants"')}}
ensure
 ActiveSupport::Notifications.unsubscribe(sub) if sub
end
baseline=measured(browser)
4.times{|i|quiet_room(Rooms::Closed,"Extra quiet #{i}",viewer)}
[394959859,712064548].each{|id| Rooms::Direct.create_for({creator:viewer},users:[viewer,User.find(id)])}
quiet_room(Rooms::Board,'Extra quiet board',viewer)
4.times{|i|quiet_room(Rooms::Stage,"Extra quiet stage #{i}",viewer)}
more=measured(browser)
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),rooms:{channel:room.id,board:board.id,direct:direct.id,group:group.id},rows:rows,queries:{baseline:baseline,more:more})
warn "Rails original sidebar oracle: #{rows.size} routed responses"
