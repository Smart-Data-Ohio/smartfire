# Real request bodies and committed handoff writes; no response masks or helper replacements.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
hashes = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/work/source-hashes.json')))
hashes.each { |file, hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator'] = ->(_) { 'NONCE' }
ActiveJob::Base.queue_adapter = :test
module HumanWorkGoldenTokens
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
ApplicationController.prepend(HumanWorkGoldenTokens)
Threads::Work::HandoffsController.skip_before_action :verify_authenticity_token
Threads::Work::LinksController.skip_before_action :verify_authenticity_token
Rails.logger = ActiveSupport::Logger.new($stderr)
ApplicationController.logger = Rails.logger
Rails.application.env_config['action_dispatch.logger'] = Rails.logger

# Restore the private seed between cases, outside a transaction: source after-commit callbacks
# run for every actual handoff, including ledger, history, activity and audit publication.
conn = ActiveRecord::Base.connection
tables = %w[channel_threads work_thread_links work_thread_events work_handoffs audit_logs agent_events activity_items memberships thread_memberships agent_grants agents users events agent_steps github_pull_requests google_accounts sqlite_sequence]
snapshot = tables.flat_map do |table|
  columns = conn.columns(table).map(&:name)
  conn.execute("SELECT #{columns.map { |column| "quote(#{conn.quote_column_name(column)})" }.join(',')} FROM #{table}").map do |row|
    "INSERT INTO #{table}(#{columns.join(',')}) VALUES(#{row.values.join(',')})"
  end
end
restore = lambda do
  conn.execute('PRAGMA foreign_keys=OFF')
  tables.each { |table| conn.execute("DELETE FROM #{table}") }
  snapshot.each { |sql| conn.execute(sql) }
  conn.execute('PRAGMA foreign_keys=ON')
  ActiveSupport::IsolatedExecutionState.clear
end
now = '2026-03-02 16:00:00'
common = [
  "UPDATE channel_threads SET work_status=NULL,work_owner_id=NULL",
  "DELETE FROM work_thread_events", "DELETE FROM work_thread_links",
  "DELETE FROM memberships WHERE room_id=654632876 AND user_id=149087659",
  "DELETE FROM agent_grants WHERE agent_id=773018776",
  "INSERT OR IGNORE INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(699448332,712064548,'mentions','#{now}','#{now}')",
  "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,created_at,updated_at,last_activity_at) VALUES(90,699448332,127326141,'Ship <&> it','in_progress',127326141,'#{now}','#{now}','#{now}'),(91,486777696,149087659,'Channel <&> work','blocked',394959859,'#{now}','2026-03-02 15:00:00','#{now}'),(92,486777696,149087659,'Completed work','done',712064548,'#{now}','2026-03-02 14:00:00','#{now}'),(93,654632876,127326141,'Hidden work','planned',NULL,'#{now}','2026-03-02 17:00:00','#{now}'),(94,486777696,149087659,'Ordinary chat',NULL,NULL,'#{now}','#{now}','#{now}')",
  "INSERT INTO thread_memberships(thread_id,user_id,involvement,joined_at,created_at,updated_at) VALUES(90,127326141,'everything','#{now}','#{now}','#{now}'),(91,149087659,'everything','#{now}','#{now}','#{now}')",
  "INSERT INTO work_thread_links(id,channel_thread_id,created_by_id,kind,url,title,created_at,updated_at) VALUES(900,90,127326141,'drive_file','https://drive.google.com/file/d/abc123','Spec <&> doc','#{now}','#{now}'),(901,91,149087659,'drive_file','https://docs.google.com/document/d/abc123',NULL,'#{now}','#{now}')"
]
%w[read_messages post_messages manage_threads].each do |cap|
  common << "INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(773018776,699448332,127326141,'#{cap}','#{now}','#{now}')"
end
cases = []
%w[open done all agents boards bogus].each do |state|
  %w[html json].each do |format|
    cases << ["work-#{state}-#{format}",149087659,'get',"/work#{format == 'json' ? '.json' : ''}?state=#{state}",{},[],{}]
  end
end
%w[open done all agents boards].each do |state|
  cases << ["work-empty-#{state}",149087659,'get',"/work?state=#{state}",{},["UPDATE channel_threads SET work_status=NULL"],{}]
end
cases += [
 ['link-drive-title',127326141,'post','/threads/90/work/links.turbo_stream',{kind:'drive_file',drive_url:'https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view'},[],{},200],
 ['link-drive-title-forbidden',127326141,'post','/threads/90/work/links.turbo_stream',{kind:'drive_file',drive_url:'https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view'},[],{},200],
  ['work-frame',149087659,'get','/work?state=all',{},[],{'Turbo-Frame'=>'main'}],
  ['work-unavailable',149087659,'get','/work?state=agents',{},["UPDATE agents SET suspended_at='#{now}' WHERE id=773018776"],{}],
  ['handoff-manager',127326141,'get','/threads/90/work/handoff/new',{},[],{}],
  ['handoff-owner',712064548,'get','/threads/90/work/handoff/new',{},["UPDATE channel_threads SET work_owner_id=712064548 WHERE id=90"],{}],
  ['handoff-plain-member',712064548,'get','/threads/90/work/handoff/new',{},[],{}],
  ['handoff-nonmember',773523953,'get','/threads/90/work/handoff/new',{},[],{}],
  ['handoff-untracked',149087659,'get','/threads/94/work/handoff/new',{},[],{}],
  ['handoff-frame',127326141,'get','/threads/90/work/handoff/new',{},[],{'Turbo-Frame'=>'main'}],
  ['handoff-missing-read',127326141,'get','/threads/90/work/handoff/new',{},["UPDATE agent_grants SET revoked_at='#{now}' WHERE capability='read_messages' AND agent_id=773018776"],{}],
  ['handoff-unknown',127326141,'get','/threads/123456/work/handoff/new',{},[],{}],
  ['channel-work-show',149087659,'get','/rooms/486777696/threads/91',{},[],{}],
  ['channel-work-pane',149087659,'get','/rooms/486777696/threads/91/content',{},[],{}],
  ['channel-work-history',149087659,'get','/rooms/486777696/threads/91',{},["INSERT INTO work_thread_events(channel_thread_id,actor_id,event_type,from_status,to_status,from_owner_name,to_owner_name,metadata,created_at,updated_at) VALUES(91,149087659,'work_handoff','blocked','blocked','David','Bender','{\"handoff_summary\":\"Ready <&> now\",\"handoff_links_count\":1,\"handoff_questions_count\":2}','#{now}','#{now}'),(91,NULL,'work_update','planned','blocked','David','Bender','{\"note\":\"Wait <&> see\"}','#{now}','#{now}')"],{}]
]
handoff = {receiver_agent_id:773018776,summary:'Halfway <&> there',links:" https://example.com/spec\r\nhttps://example.com/spec\n",open_questions:['Which API?']}
[
 ['handoff-create-json',127326141,handoff,[],201],
 ['handoff-create-owner',712064548,handoff,["UPDATE channel_threads SET work_owner_id=712064548 WHERE id=90"],201],
 ['handoff-manager-other-owner',127326141,handoff,["UPDATE channel_threads SET work_owner_id=712064548 WHERE id=90"],201],
 ['handoff-create-forbidden',712064548,{receiver_agent_id:0,summary:''},[],403],
 ['handoff-create-hidden',773523953,handoff,[],404],
 ['handoff-receiver-missing',127326141,{receiver_agent_id:0,summary:'Hi'},[],422],
 ['handoff-receiver-read',127326141,handoff,["UPDATE agent_grants SET revoked_at='#{now}' WHERE capability='read_messages' AND agent_id=773018776"],422],
 ['handoff-receiver-manage',127326141,handoff,["UPDATE agent_grants SET revoked_at='#{now}' WHERE capability='manage_threads' AND agent_id=773018776"],422],
 ['handoff-receiver-outside',127326141,handoff,["DELETE FROM memberships WHERE room_id=699448332 AND user_id=394959859"],422],
 ['handoff-receiver-owner',127326141,handoff,["UPDATE channel_threads SET work_owner_id=394959859 WHERE id=90"],422],
 ['handoff-summary-blank',127326141,handoff.merge(summary:''),[],422],
 ['handoff-summary-limit',127326141,handoff.merge(summary:'x'*2001),[],422],
 ['handoff-links-invalid',127326141,handoff.merge(links:['ftp://example.test']),[],422],
 ['handoff-false-list',127326141,handoff.merge(links:false,open_questions:false),[],201]
].each do |name,user,input,setup,status|
 cases << [name,user,'post','/threads/90/work/handoff.json',input,setup,{},status]
end
[[ 'array', [0,773018776], 201 ], [ 'float', 773018776.75, 201 ], [ 'hash', {value:773018776}, 422 ],
 ['single-nested-array', [nil,[773018776]], 201], ['multi-nested-array', [[773018776],0], 422],
 ['underscores', '773_018776tail', 201], ['unicode-space', "\u00a0773018776", 422]].each do |shape,value,status|
 cases << ["handoff-receiver-#{shape}",127326141,'post','/threads/90/work/handoff.json',handoff.merge(receiver_agent_id:value),[],{},status]
end
cases << ['handoff-invalid-html',127326141,'post','/threads/90/work/handoff',{receiver_agent_id:773018776,summary:''},[],{},422]
cases << ['handoff-create-html',127326141,'post','/threads/90/work/handoff',handoff,[],{},302]
cases << ['handoff-create-untracked',149087659,'post','/threads/94/work/handoff.json',handoff,[],{},422]
cases += [
 ['link-event-array',149087659,'post','/threads/91/work/links.turbo_stream',{kind:'event',event_id:[0,411254270]},[],{},200],
 ['link-event-float',149087659,'post','/threads/91/work/links.turbo_stream',{kind:'event',event_id:411254270.75},[],{},200],
 ['link-event-hash',149087659,'post','/threads/91/work/links.turbo_stream',{kind:'event',event_id:{value:411254270}},[],{},404],
 ['link-event-underscores',149087659,'post','/threads/91/work/links.turbo_stream',{kind:'event',event_id:'411_254270tail'},[],{},200],
 ['links-panel',127326141,'get','/threads/90/work/links',{},[],{},200],
 ['links-panel-frame',127326141,'get','/threads/90/work/links',{},[],{'Turbo-Frame'=>'thread-panel-work-links'},200],
 ['links-plain-member',712064548,'get','/threads/90/work/links',{},[],{},200],
 ['links-hidden',773523953,'get','/threads/90/work/links',{},[],{},404],
 ['links-untracked',149087659,'get','/threads/94/work/links',{},[],{},422],
 ['links-unknown',127326141,'get','/threads/123456/work/links',{},[],{},404]
]
[
 ['link-drive',127326141,{kind:'drive_file',drive_url:'  https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view  '},[],200],
 ['link-plain-member',712064548,{kind:'drive_file',drive_url:'https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view'},[],200],
 ['link-pr',127326141,{kind:'pull_request',pull_request_url:'https://github.com/Rails/Rails/pull/12/files?diff=split'},[],200],
 ['link-event',149087659,{kind:'event',event_id:411254270},[],200],
 ['link-event-cancelled',149087659,{kind:'event',event_id:411254270},["UPDATE events SET cancelled_at='#{now}' WHERE id=411254270"],200],
 ['link-event-other-room',127326141,{kind:'event',event_id:411254270},[],404],
 ['link-event-blank',127326141,{kind:'event',event_id:''},[],422],
 ['link-pr-invalid',127326141,{kind:'pull_request',pull_request_url:'not a URL'},[],422],
 ['link-drive-invalid',127326141,{kind:'drive_file',drive_url:'https://example.test/not-drive'},[],422],
 ['link-kind-invalid',127326141,{kind:'bogus'},[],422],
 ['link-drive-duplicate',127326141,{kind:'drive_file',drive_url:'https://drive.google.com/file/d/abcdef123456'},["UPDATE work_thread_links SET url='https://drive.google.com/file/d/abcdef123456' WHERE id=900"],422],
 ['link-hidden-write',773523953,{kind:'bogus'},[],404]
].each do |name,user,input,setup,status|
 thread=%w[link-event link-event-cancelled].include?(name) ? 91 : 90
 cases << [name,user,'post',"/threads/#{thread}/work/links.turbo_stream",input,setup,{},status]
end
cases += [
 ['link-create-html',127326141,'post','/threads/90/work/links',{kind:'drive_file',drive_url:'https://docs.google.com/document/d/abcdefghij'},[],{},302],
 ['link-invalid-html',127326141,'post','/threads/90/work/links',{kind:'drive_file',drive_url:'no'},[],{},302],
 ['link-delete-stream',712064548,'delete','/threads/90/work/links/900.turbo_stream',{},[],{},200],
 ['link-delete-html',712064548,'delete','/threads/90/work/links/900',{},[],{},302],
 ['link-delete-missing',127326141,'delete','/threads/90/work/links/123456.turbo_stream',{},[],{},404],
 ['link-delete-other-thread',127326141,'delete','/threads/90/work/links/901.turbo_stream',{},[],{},404],
 ['link-delete-hidden',773523953,'delete','/threads/90/work/links/900.turbo_stream',{},[],{},404]
]
cases += [
 ['link-create-untracked',149087659,'post','/threads/94/work/links.turbo_stream',{kind:'bogus'},[],{},422],
 ['link-delete-untracked',149087659,'delete','/threads/94/work/links/900.turbo_stream',{},[],{},422],
 ['link-delete-event',149087659,'delete','/threads/91/work/links/902.turbo_stream',{},["INSERT INTO work_thread_links(id,channel_thread_id,created_by_id,kind,event_id,created_at,updated_at) VALUES(902,91,149087659,'event',411254270,'#{now}','#{now}')"],{},200],
 ['link-delete-pr',127326141,'delete','/threads/90/work/links/902.turbo_stream',{},["INSERT INTO github_pull_requests(id,owner,repo,number,created_at,updated_at) VALUES(900083005,'rails','rails',90999999,'#{now}','#{now}')", "INSERT INTO work_thread_links(id,channel_thread_id,created_by_id,kind,github_pull_request_id,created_at,updated_at) VALUES(902,90,127326141,'pull_request',900083005,'#{now}','#{now}')"],{},200],
 ['link-delete-html-back',712064548,'delete','/threads/90/work/links/900',{},[],{'Referer'=>'https://campfire.test/work?state=all'},302]
]
# Record Google's HTTP boundary, leaving Client, account policy and owned code unchanged.
google_reply = nil
http = Object.new
http.define_singleton_method(:get) do |path, headers|
 raise "unexpected Drive credential/target" unless path.start_with?('/drive/v3/files/1AbcDefGhIjKlMnOpQrSt?') && headers['Authorization'] == ['Bearer','access-token'].join(' ')
 status, body = google_reply
 response = (status == 200 ? Net::HTTPOK : Net::HTTPForbidden).new('1.1', status.to_s, 'fixture')
 response.body = body.to_json
 response.instance_variable_set(:@read, true)
 response
end
transport = Module.new
transport.define_method(:start) do |host,*args,**options,&block|
 host == 'www.googleapis.com' && google_reply ? block.call(http) : super(host,*args,**options,&block)
end
Net::HTTP.singleton_class.prepend(transport)
rows = cases.map do |name,user_id,method,path,input,setup,headers,status|
 restore.call
 (common+setup).each { |sql| conn.execute(sql) }
 google_reply = if name == 'link-drive-title' then [200,{name:'Plan <&> title'}]
 elsif name == 'link-drive-title-forbidden' then [403,{error:{message:'private file'}}] end
 if google_reply
  ENV['GOOGLE_CLIENT_ID']='test-client-id'; ENV['GOOGLE_CLIENT_SECRET']='FAKE-google-client-secret'
  GoogleAccount.where(user_id: user_id).delete_all
  GoogleAccount.create!(user_id:user_id,email:'david@gmail.test',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:1.hour.from_now,scopes:"#{Google::Client::CALENDAR_SCOPE} #{Google::Client::DRIVE_SCOPE}")
 else
  ENV.delete('GOOGLE_CLIENT_ID'); ENV.delete('GOOGLE_CLIENT_SECRET')
 end
 user = User.find(user_id)
 request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
 request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at:nil).first!.token
 browser = ActionDispatch::Integration::Session.new(Rails.application)
 browser.host! 'campfire.test'
 headers = headers.merge('Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}",'User-Agent'=>'Mozilla')
 if method == 'get'
   browser.get(path, headers:headers)
 else
   browser.public_send(method,path, params:input.to_json,headers:headers.merge('Content-Type'=>'application/json'))
 end
 expected = status || (name == 'handoff-plain-member' ? 403 : %w[handoff-nonmember handoff-unknown].include?(name) ? 404 : name == 'handoff-untracked' ? 422 : 200)
 raise "#{name}: expected #{expected}, got #{browser.response.status}" unless browser.response.status == expected
 {name:,user_id:,method:,path:,input:,setup:common+setup,headers:headers.except('Cookie','User-Agent'),status:browser.response.status,body:browser.response.body,location:browser.response.headers['Location'],cache_control:browser.response.headers['Cache-Control'],content_type:browser.response.headers['Content-Type'],google_reply:}
end
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),sources:hashes,rows:)
warn "Rails human work HTTP oracle: #{rows.size} complete responses; committed handoffs; 0 masks"
