HASHES={'app/controllers/rooms/reads_controller.rb' => 'c6a6b8dd72e62c1c342bc8c44de57c90b2ce38734f2c410868efd273a5a5af6e', 'app/controllers/rooms/refreshes_controller.rb' => '43f3d103e2d37c7e04af0f6117ebec56a86d43f9276d9fec961b802796822344', 'app/models/membership.rb' => '7942db424021486f7046742095b26141031dab9da05e599e22f382d120e4ed9b'}
require 'json'
require 'digest'
HASHES.each { |path, hash| raise "reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
DAVID=127326141
ROOM=486777696
def browser(user_id)
  client=ActionDispatch::Integration::Session.new(Rails.application)
  client.host! 'campfire.test'
  if user_id
    user=User.find(user_id)
    session=user.sessions.create!(two_factor_verified_at:Time.current)
    req=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
    req.cookie_jar.signed.permanent[:session_token]={value:session.token,httponly:true,same_site: :lax}
    client.cookies['session_token']=req.cookie_jar[:session_token]
    client.get('/users/me/profile')
    ActiveSupport::IsolatedExecutionState.clear
    token=Nokogiri::HTML(client.response.body).at_css('meta[name="csrf-token"]')['content']
  end
  [client,token]
end
frames=[]
ActionCable.server.define_singleton_method(:broadcast) { |stream,message,**_| frames << {stream:stream,message:message} if stream.match?(/^user_\d+_(reads|unreads)$/) }
client,token=browser(DAVID)
roots=Room.find(ROOM).root_messages.ordered.to_a
cases={}
def record(cases,key,client,frames)
  ActiveSupport::IsolatedExecutionState.clear
  response=client.response
  membership=Membership.find_by!(room_id:ROOM,user_id:DAVID)
  cases[key]={status:response.status,location:response.headers['Location'],json:(JSON.parse(response.body) rescue nil),frames:frames.map(&:dup),unread:membership.unread?,pointer:membership.last_read_message_id}
end
[['create',:post,nil],['second',:delete,roots[1].id],['first',:delete,roots[0].id],['foreign',:delete,Room.find(699448329).root_messages.first.id],['missing',:delete,9999999999]].each do |key,method,message_id|
  frames.clear
  client.public_send(method,"/rooms/#{ROOM}/read",params:message_id ? {message_id:message_id} : {},headers:{'X-CSRF-Token'=>token,'Accept'=>'application/json'})
  record(cases,key,client,frames)
end
[['outsider',712064548],['anonymous',nil]].each do |key,id|
  c,t=browser(id)
  frames.clear
  c.post("/rooms/#{ROOM}/read",headers:{'X-CSRF-Token'=>t.to_s,'Accept'=>'application/json'})
  record(cases,key,c,frames)
end
frames.clear
anon,_=browser(nil)
anon.post("/rooms/#{ROOM}/read?bot_key=394959859-BenderToken1",headers:{'Accept'=>'application/json'})
record(cases,'bot',anon,frames)
Membership.where(room_id:ROOM,user_id:DAVID).update_all(unread_at:Time.current,last_read_message_id:nil)
Room.find(ROOM).update_columns(deleted_at:Time.current)
frames.clear
client.post("/rooms/#{ROOM}/read",headers:{'X-CSRF-Token'=>token,'Accept'=>'application/json'})
record(cases,'deleted',client,frames)
refresh={}
['text/vnd.turbo-stream.html','text/html','application/json'].each do |accept|
  client.get('/rooms/699448329/refresh',params:{since:(Time.current.to_f*1000).to_i},headers:{'Accept'=>accept})
  ActiveSupport::IsolatedExecutionState.clear
  refresh[accept]={status:client.response.status,body:client.response.body}
end
puts JSON.pretty_generate({reference: ENV.fetch('PARITY_REFERENCE_SHA'),root_ids:[roots.first.id,roots[1].id,roots.last.id],cases:cases,refresh:refresh})
warn "Rails reads/refresh oracle: #{cases.size} read cases, #{refresh.size} quiet refresh formats; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
