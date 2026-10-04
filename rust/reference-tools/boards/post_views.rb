# Actual Rails HTTP bodies for board-post forms, pages and conversation panes.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
hashes=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/source-hashes.json')))
hashes.merge!(JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/boards/post-source-hashes.json'))))
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
rows=[]
fixtures=[
 ['new-admin',127326141,'/rooms/699448332/threads/new',[],{}],
 ['new-member',149087659,'/rooms/699448332/threads/new',[],{}],
 ['new-frame',127326141,'/rooms/699448332/threads/new',[],{'Turbo-Frame'=>'main'}],
 ['new-prefilled',127326141,'/rooms/699448332/threads/new?thread[first_message]=Brief%20%3Cnow%3E',[],{}],
 ['new-channel',127326141,'/rooms/486777696/threads/new',[],{}],
 ['new-nonmember',712064548,'/rooms/699448332/threads/new',[],{}],
 ['post-admin',127326141,'/rooms/699448332/threads/4',[],{}],
 ['post-member',149087659,'/rooms/699448332/threads/4',[],{}],
 ['post-frame',127326141,'/rooms/699448332/threads/4',[],{'Turbo-Frame'=>'main'}],
 ['post-closed',127326141,'/rooms/699448332/threads/4',["UPDATE channel_threads SET closed_at='2026-03-02 16:00:00' WHERE id=4"],{}],
 ['post-locked',127326141,'/rooms/699448332/threads/4',["UPDATE channel_threads SET closed_at='2026-03-02 16:00:00',locked_at='2026-03-02 16:00:00' WHERE id=4"],{}],
 ['post-empty',127326141,'/rooms/699448332/threads/4',["DELETE FROM messages WHERE thread_id=4","DELETE FROM thread_tags WHERE channel_thread_id=4","UPDATE channel_threads SET work_owner_id=NULL WHERE id=4"],{}],
 ['post-unavailable',149087659,'/rooms/699448332/threads/4',["UPDATE users SET status=1 WHERE id=127326141"],{}],
 ['post-result',127326141,'/rooms/699448332/threads/4',["UPDATE channel_threads SET result_markdown='## Shipped\n\n**Outcome** <script>alert(1)</script> :sparkles:',result_updated_at='2026-03-02 16:00:00',result_updated_by_id=149087659,run_url='https://example.test/runs/1' WHERE id=4"],{}],
 ['post-history',127326141,'/rooms/699448332/threads/4',["INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id,from_owner_name,to_owner_name,metadata,created_at,updated_at) VALUES(4,149087659,'work_update','planned','done',NULL,127326141,NULL,'David','{\"note\":\"Ship <now>\"}','2026-03-02 16:00:00','2026-03-02 16:00:00')","INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,metadata,created_at,updated_at) VALUES(4,NULL,'result_updated','done','done','{}','2026-03-02 16:00:00','2026-03-02 16:00:00')","INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_owner_name,to_owner_name,metadata,created_at,updated_at) VALUES(4,149087659,'work_handoff','David','Bender','{\"handoff_summary\":\"Ready & waiting\",\"handoff_links_count\":1,\"handoff_questions_count\":2}','2026-03-02 16:00:00','2026-03-02 16:00:00')"],{}],
 ['post-drive',127326141,'/rooms/699448332/threads/4',["INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,url,title,created_at,updated_at) VALUES(4,127326141,'drive_file','https://drive.google.com/file/d/boards1234567','Result <sheet>','2026-03-02 16:00:00','2026-03-02 16:00:00')"],{}],
 ['post-agent',127326141,'/rooms/699448332/threads/6',[],{}],
 ['post-owner',712064548,'/rooms/699448332/threads/4',["INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,712064548,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')","UPDATE channel_threads SET work_owner_id=712064548 WHERE id=4"],{}],
 ['post-observer',773523953,'/rooms/699448332/threads/4',["INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,773523953,'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')"],{}],
 ['new-agent-suspended',127326141,'/rooms/699448332/threads/new',["UPDATE agents SET suspended_at='2026-03-02 16:00:00' WHERE id=773018776"],{}],
 ['new-agent-grant-revoked',127326141,'/rooms/699448332/threads/new',["INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(773018776,699448332,'post_messages',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"],{}],
 ['post-agent-unavailable',127326141,'/rooms/699448332/threads/6',["UPDATE agents SET suspended_at='2026-03-02 16:00:00' WHERE id=773018776"],{}],
 ['post-pr-public',127326141,'/rooms/699448332/threads/4',["UPDATE github_pull_requests SET private=0,title='Public <title>' WHERE id=1","INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,github_pull_request_id,created_at,updated_at) VALUES(4,127326141,'pull_request',1,'2026-03-02 16:00:00','2026-03-02 16:00:00')"],{}],
 ['post-pr-private',127326141,'/rooms/699448332/threads/4',["UPDATE github_pull_requests SET private=1,title='Private title must stay hidden' WHERE id=1","INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,github_pull_request_id,created_at,updated_at) VALUES(4,127326141,'pull_request',1,'2026-03-02 16:00:00','2026-03-02 16:00:00')"],{}],
 ['post-events',127326141,'/rooms/699448332/threads/4',["INSERT INTO events(id,room_id,organizer_id,title,starts_at,ends_at,time_zone,created_at,updated_at) VALUES(9000300001,699448332,127326141,'Review & plan','2026-03-03 16:00:00','2026-03-03 17:00:00','America/New_York','2026-03-02 16:00:00','2026-03-02 16:00:00'),(9000300002,699448332,127326141,'Next <event>','2026-03-04 16:00:00',NULL,'America/New_York','2026-03-02 16:00:00','2026-03-02 16:00:00')","INSERT INTO work_thread_links(channel_thread_id,created_by_id,kind,event_id,created_at,updated_at) VALUES(4,127326141,'event',9000300001,'2026-03-02 16:00:00','2026-03-02 16:00:00')"],{}],
 ['post-nonmember' ,712064548,'/rooms/699448332/threads/4',[],{}],
 ['pane',127326141,'/rooms/699448332/threads/4/content',[],{}],
 ['pane-anchor',149087659,'/rooms/699448332/threads/4/content?message_id=935962049',[],{}],
 ['pane-empty',127326141,'/rooms/699448332/threads/4/content',["DELETE FROM messages WHERE thread_id=4"],{}]
]
fixtures.each do |name,user_id,path,setup,headers|
 ActiveRecord::Base.transaction do
  setup.each { |sql| ActiveRecord::Base.connection.execute(sql) }
  user=User.find(user_id)
  request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
  browser.get(path,headers:headers.merge('Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",'User-Agent'=>'Mozilla'))
  expected_status=%w[new-channel new-nonmember post-nonmember].include?(name) ? 404 : 200
  raise "unexpected Rails response #{name}: #{browser.response.status}" unless browser.response.status==expected_status
  rows << {name:,user_id:,path:,setup:,headers:,status:browser.response.status,location:browser.response.headers['Location'],html:browser.response.body}
  raise ActiveRecord::Rollback
 end
 ActiveSupport::IsolatedExecutionState.clear
end
sources=%w[app/models/channel_thread.rb app/views/channel_threads/new.html.erb app/views/channel_threads/_board_post.html.erb app/views/channel_threads/_conversation.html.erb app/views/threads/work/links/_box.html.erb app/views/threads/work/links/_link.html.erb app/views/threads/work/links/_status.html.erb]
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),board_reference:ENV.fetch("PARITY_REFERENCE_SHA"),sources:sources.to_h { |f| [f,Digest::SHA256.file(Rails.root.join(f)).hexdigest] },rows:)
warn "Rails board post read oracle: #{rows.size} complete HTTP responses; plain pinned image; no masks"
