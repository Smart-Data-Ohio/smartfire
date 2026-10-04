# Actual Rails HTTP bodies for board-post forms, pages and conversation panes.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
hashes=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/source-hashes.json')))
hashes.merge!(JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/post-source-hashes.json'))))
hashes.merge!(JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/write-source-hashes.json'))))
hashes.each { |file,hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==hash }
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_) {'NONCE'}
ActiveJob::Base.queue_adapter=:test
module PostGoldenTokens
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
ApplicationController.prepend(PostGoldenTokens)
Rails.logger = ActiveSupport::Logger.new($stderr)
ApplicationController.logger = Rails.logger
Rails.application.env_config['action_dispatch.logger'] = Rails.logger
ChannelThreadsController.skip_before_action :verify_authenticity_token
rows=[]
board=699448332
admin=127326141
owner=712064548
common=["INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,712064548,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')", "DELETE FROM agent_grants WHERE agent_id=773018776"]
%w[read_messages post_messages manage_threads].each do |cap|
 common << "INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(773018776,699448332,127326141,'#{cap}','2026-03-02 16:00:00','2026-03-02 16:00:00')"
end
common.each { |sql|ActiveRecord::Base.connection.execute(sql) }
[
 ['auto-human','post','/rooms/699448332/threads.json',{thread:{name:'Auto human',tags:'auto-human'}},712064548,[]],
 ['auto-agent','post','/rooms/699448332/threads.json',{thread:{name:'Auto agent',tags:'auto-agent'}},394959859,[]],
 ['auto-owned','post','/rooms/699448332/threads.json',{thread:{name:'Already owned',tags:'auto-owned',work_owner_id:712064548}},394959859,[]],
 ['auto-edit','patch','/rooms/699448332/threads/4.json',{thread:{tags:'auto-edit'}},712064548,["UPDATE channel_threads SET work_owner_id=NULL WHERE id=4"]]
].each do |name,method,path,input,assignee,extra|
 sequences=ActiveRecord::Base.connection.execute('SELECT name,seq FROM sqlite_sequence').map { |row|"UPDATE sqlite_sequence SET seq=#{row['seq']} WHERE name='#{row['name']}'" }
 rule="INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(699448332,'#{name}',#{assignee},127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00')"
 (extra+[rule]).each { |sql|ActiveRecord::Base.connection.execute(sql) }
 user=User.find(admin)
 request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
 request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
 browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
 browser.public_send(method,path,params:input.to_json,headers:{'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",'Content-Type'=>'application/json','User-Agent'=>'Mozilla'})
 expected=method=='post' ? 201 : 200
 raise "unexpected Rails response #{name}: #{browser.response.status}" unless browser.response.status==expected
 rows << {name:,method:,path:,input:,setup:common+sequences+extra+[rule],status:browser.response.status,body:browser.response.body}
 ActiveSupport::IsolatedExecutionState.clear
end
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),rows:)
warn "Rails tag auto-assignment HTTP oracle: #{rows.size} complete committed responses; 0 masks"
